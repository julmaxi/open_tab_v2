use std::fmt::Debug;
use std::hash::Hash;
use std::iter::{zip, self};
use std::collections::{HashMap, HashSet};


use crate::derived_models::BreakNodeBackgroundInfo;
use crate::domain::entity::LoadEntity;
use crate::domain::tournament_plan_node::PlanNodeType;
use crate::info::{get_tournament_teams_members, TournamentParticipantsInfo};
use serde::{Serialize, Deserialize};

use sea_orm::{prelude::*, QuerySelect};
use crate::{prelude::*, domain};

use crate::schema::{self, speaker};

use itertools::Itertools;

use ordered_float::OrderedFloat;
pub use sea_orm::prelude::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabView {
    pub num_rounds: u32,
    pub team_index: HashMap<Uuid, usize>,
    pub speaker_index: HashMap<Uuid, usize>,
    pub team_tab: Vec<TeamTabEntry>,
    pub speaker_tab: Vec<SpeakerTabEntry>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTabEntry {
    pub rank: u32,
    pub team_uuid: Uuid,
    pub total_score: f64,
    pub avg_score: Option<f64>,
    pub detailed_scores: Vec<Option<TeamTabEntryDetailedScore>>,
    pub member_ranks: Vec<u32>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTabEntryDetailedScore {
    pub team_score: Option<f64>,
    pub speaker_score: f64,
    pub role: TeamRoundRole
}

impl TeamTabEntryDetailedScore {
    pub fn total_score(&self) -> f64 {
        match self.team_score {
            Some(team_score) => team_score + self.speaker_score,
            None => self.speaker_score
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeamRoundRole {
    Government,
    Opposition,
    NonAligned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTabEntry {
    pub rank: u32,
    pub speaker_uuid: Uuid,
    pub team_uuid: Uuid,
    pub total_score: f64,
    pub avg_score: Option<f64>,
    pub detailed_scores: Vec<Option<SpeakerTabEntryDetailedScore>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTabEntryDetailedScore {
    pub score: f64,
    pub team_role: TeamRoundRole,
    pub speech_position: u8
}

impl TabView {
    pub async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(
            schema::tournament_round::Column::TournamentId.eq(tournament_uuid)
        ).all(db).await?;

        let team_members = get_tournament_teams_members(db, tournament_uuid).await?;

        Self::load_from_rounds(db, rounds.into_iter().map(|r| r.uuid).collect(), &team_members).await
    }

    pub async fn load_from_rounds<C>(db: &C, round_ids: Vec<Uuid>, team_members: &HashMap<Uuid, Vec<Uuid>>) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let num_round_ids = round_ids.len();
        let rounds_with_debates = schema::tournament_round::Entity::find()
        .find_with_related(schema::tournament_debate::Entity)
        .filter(schema::tournament_round::Column::Uuid.is_in(round_ids))
        .all(db).await?;

        let relevant_ballot_ids = rounds_with_debates.iter().flat_map(|(_, debates)| debates.iter().map(|d| d.ballot_id)).collect_vec();
        let ballots = domain::ballot::Ballot::get_many(db, relevant_ballot_ids).await?;
        let ballots_by_id = ballots.iter().map(|b| (b.uuid, b)).collect::<HashMap<_, _>>();
        
        // Include uuid to ensure order is always stable, even when indices overlap
        let round_order = rounds_with_debates.iter().map(|(round, _)| round).sorted_by_key(|r| (r.index, r.uuid)).map(|r| r.uuid).collect_vec();

        let speaker_teams = team_members.iter().flat_map(|(team_id, members)| {
            members.iter().map(|member| (*member, *team_id))
        }).collect::<HashMap<_, _>>();

        let mut team_detailed_scores = team_members.keys().map(|k| (*k, HashMap::new())).collect::<HashMap<_, _>>();
        let mut speaker_detailed_scores = team_members.values().flat_map(|m| m.iter().map(|k| (*k, HashMap::new()))).collect::<HashMap<_, _>>();
        for (round, debates) in rounds_with_debates {
            let mut non_aligned_teams = HashSet::new();
            let mut non_aligned_teams_opt_out_count = HashMap::new();
            let mut non_aligned_teams_individual_scores = HashMap::new();
            for debate in debates.iter() {
                let ballot = ballots_by_id.get(&debate.ballot_id).expect("Guaranteed by db constraints");

                for role in vec![TeamRoundRole::Government, TeamRoundRole::Opposition] {
                    let (team_score, speaker_scores) = Self::detail_score_for_debate_side(&ballot, &role);

                    let team_id = match &role {
                        TeamRoundRole::Government => ballot.government.team,
                        TeamRoundRole::Opposition => ballot.opposition.team,
                        _ => unreachable!()
                    };

                    if let Some(team_id) = team_id {
                        let team_entries = team_detailed_scores.entry(team_id).or_insert_with(|| HashMap::new());
                        if team_entries.contains_key(&round.uuid) {
                            return Err(anyhow::Error::msg("Team can not be in the same round twice"));
                        }
                        team_entries.insert(round.uuid, TeamTabEntryDetailedScore {
                            team_score,
                            speaker_score: speaker_scores.into_iter().sum(),
                            role
                        });
                    }
                }

                for speech in &ballot.speeches {
                    if let Some(speaker) = speech.speaker {
                        let speaker_team = speaker_teams.get(&speaker);
                        
                        let score = if speech.is_opt_out {
                            match speech.role {
                                SpeechRole::Government | SpeechRole::Opposition => {},
                                SpeechRole::NonAligned => {
                                    if let Some(speaker_team) = speaker_team {
                                        *non_aligned_teams_opt_out_count.entry(*speaker_team).or_insert(0) += 1;
                                    }
                                }
                            }
                            0.0
                        }
                        else {
                            let score = speech.speaker_score();
                            if let Some(score) = score {
                                score
                            }
                            else {
                                continue;
                            }
                        };
    
                        match speech.role {
                            SpeechRole::Government | SpeechRole::Opposition => {},
                            SpeechRole::NonAligned => {
                                if let Some(speaker_team) = speaker_team {
                                    non_aligned_teams.insert(*speaker_team);

                                    if !speech.is_opt_out {
                                        non_aligned_teams_individual_scores.entry(speaker_team).or_insert_with(|| vec![]).push(score);
                                    }
                                }
                            },
                        }

                        if !speech.is_opt_out {
                            let speaker_entries = speaker_detailed_scores.entry(speaker).or_insert_with(|| HashMap::new());
                            if speaker_entries.contains_key(&round.uuid) {
                                return Err(anyhow::Error::msg(format!("Speaker {} can not be in the same round twice", speaker)));
                            }
                            speaker_entries.insert(round.uuid, SpeakerTabEntryDetailedScore {
                                score,
                                team_role: match speech.role {
                                    SpeechRole::Government => TeamRoundRole::Government,
                                    SpeechRole::Opposition => TeamRoundRole::Opposition,
                                    SpeechRole::NonAligned => TeamRoundRole::NonAligned
                                },
                                speech_position: speech.position
                            });    
                        }
                    }
                }
            }

            let empty = vec![];
            for team_id in non_aligned_teams {
                let scores = non_aligned_teams_individual_scores.get(&team_id).unwrap_or(&empty);
                let team_entries = team_detailed_scores.entry(team_id).or_insert_with(|| HashMap::new());
                if team_entries.contains_key(&round.uuid) {
                    return Err(anyhow::Error::msg("Team can not be in the same round as both team and non-aligned."));
                }

                let mut speaker_score = scores.iter().sum();

                speaker_score += scores.iter().min_by_key(|s| OrderedFloat(**s)).map(|s| *s).unwrap_or(0.0) * (*non_aligned_teams_opt_out_count.get(&team_id).unwrap_or(&0) as f64);
                
                team_entries.insert(round.uuid, TeamTabEntryDetailedScore {
                    team_score: None,
                    speaker_score: speaker_score,
                    role: TeamRoundRole::NonAligned
                });
            }
        }

        let mut speaker_tab = speaker_detailed_scores.into_iter().map(
            |(speaker_id, per_round_score)| {
                SpeakerTabEntry {
                    rank: 0,
                    speaker_uuid: speaker_id,
                    team_uuid: speaker_teams.get(&speaker_id).cloned().unwrap_or_default(),
                    total_score: per_round_score.values().map(|s| s.score).sum(),
                    avg_score: if per_round_score.values().len() > 0 {
                        Some(per_round_score.values().map(|s| s.score).sum::<f64>() /  per_round_score.values().len() as f64)
                    }
                    else {
                        None
                    },
                    detailed_scores: round_order.iter().map(|r| per_round_score.get(&r).cloned()).collect_vec(),
                }
            }
        ).sorted_by_key(|s| -OrderedFloat(s.total_score)).collect_vec();

        let mut prev_val = None;
        let mut prev_rank = 0;
        let mut speaker_rank_map = HashMap::new();
        for (i, speaker) in speaker_tab.iter_mut().enumerate() {
            match prev_val {
                Some(prev_val) if prev_val == speaker.total_score => {
                    speaker.rank = prev_rank;
                },
                _ => {
                    speaker.rank = i as u32;
                }
            }

            speaker_rank_map.insert(speaker.speaker_uuid, speaker.rank);
            prev_val = Some(speaker.total_score);
            prev_rank = speaker.rank;
        }

        let mut team_tab = team_detailed_scores.into_iter().map(
            |(team_id, per_round_score)| {
                TeamTabEntry {
                    rank: 0,
                    team_uuid: team_id,
                    total_score: per_round_score.values().map(|s| s.total_score()).sum(),
                    avg_score: if per_round_score.values().len() > 0 {
                        Some(per_round_score.values().map(|s| s.total_score()).sum::<f64>() /  per_round_score.values().len() as f64)
                    }
                    else {
                        None
                    },
                    detailed_scores: round_order.iter().map(|r| per_round_score.get(&r).cloned()).collect_vec(),
                    member_ranks: team_members.get(&team_id).map(|members| {
                        members.iter().filter_map(|member| speaker_rank_map.get(member).cloned()).sorted().collect_vec()
                    }).unwrap_or(vec![])
                }
            }
        ).sorted_by_key(|s| -OrderedFloat(s.total_score)).collect_vec();

        let mut prev_val = None;
        let mut prev_rank = 0;
        for (i, team) in team_tab.iter_mut().enumerate() {
            match prev_val {
                Some(prev_val) if prev_val == team.total_score => {
                    team.rank = prev_rank;
                },
                _ => {
                    team.rank = i as u32;
                }
            }

            speaker_rank_map.insert(team.team_uuid, team.rank);
            prev_val = Some(team.total_score);
            prev_rank = team.rank;
        }

        let team_index = team_tab.iter().enumerate().map(|(i, t)| (t.team_uuid, i)).collect::<HashMap<_, _>>();
        let speaker_index = speaker_tab.iter().enumerate().map(|(i, t)| (t.speaker_uuid, i)).collect::<HashMap<_, _>>();

        Ok(
            TabView { team_tab, speaker_tab, num_rounds: num_round_ids as u32, team_index, speaker_index }
        )
    }

    fn detail_score_for_debate_side(ballot: &Ballot, team_role: &TeamRoundRole) -> (Option<f64>, Vec<f64>) {
        let (team_score, speaker_scores) = match team_role {
            TeamRoundRole::Government => (ballot.government.team_score(), ballot.government_speech_scores()),
            TeamRoundRole::Opposition => (ballot.opposition.team_score(), ballot.opposition_speech_scores()),
            TeamRoundRole::NonAligned => panic!("Can't compute team score for non-aligned speakers")
        };
        (team_score, speaker_scores)
    }
}
