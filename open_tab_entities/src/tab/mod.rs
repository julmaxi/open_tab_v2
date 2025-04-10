use std::fmt::Debug;
use std::hash::Hash;
use std::iter::{zip, self};
use std::collections::{HashMap, HashSet};

mod base;
mod augmented;
pub use base::{TabView};

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
pub use self::base::*;
pub use self::augmented::*;

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreakingAdjudicatorInfo {
    pub name: String,
    pub uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreakRelevantTabView {
    pub tab: TabView,
    pub speaker_teams: HashMap<Uuid, Uuid>,
    pub team_members: HashMap<Uuid, Vec<Uuid>>,
    pub breaking_teams: Vec<Uuid>,
    pub breaking_speakers: Vec<Uuid>,
    pub breaking_adjudicators: Vec<Uuid>
}

impl BreakRelevantTabView {
    pub async fn load_from_node<C>(db: &C, node_uuid: Uuid) -> Result<BreakRelevantTabView, anyhow::Error> where C: ConnectionTrait {
        let target_node = crate::domain::tournament_plan_node::TournamentPlanNode::get(db, node_uuid).await?;
        let break_background = BreakNodeBackgroundInfo::load_for_break_node(db, target_node.tournament_id, node_uuid).await?;
        let speaker_info = TournamentParticipantsInfo::load(db, target_node.tournament_id).await?;

        let break_id = match target_node.config {
            PlanNodeType::Break { break_id, .. } => {
                break_id
            },
            _ =>  None
        };

        let (breaking_teams, breaking_speakers, breaking_adjudicators) = match break_id {
            Some(break_id) => {
                let break_ = crate::domain::tournament_break::TournamentBreak::get(db, break_id).await?;
                (break_.breaking_teams, break_.breaking_speakers, break_.breaking_adjudicators)
            },
            None => (vec![], vec![], vec![])
        };

        let tab = TabView::load_from_rounds(
            db,
            break_background.preceding_rounds.clone(),
            &speaker_info.team_members,
        ).await?;

        Ok(BreakRelevantTabView {
            tab,
            speaker_teams: speaker_info.speaker_teams,
            team_members: speaker_info.team_members,
            breaking_teams,
            breaking_speakers,
            breaking_adjudicators
        })
    }
}



/* 
impl AugmentedTabView {
    pub async fn load_from_rounds_with_anonymity<C>(db: &C, round_ids: Vec<Uuid>, speaker_info: &super::info::TournamentParticipantsInfo, respect_anonymity: bool) -> Result<AugmentedTabView, anyhow::Error> where C: ConnectionTrait {
        let mut tab = Self::load_from_rounds(db, round_ids, speaker_info).await?;
        if respect_anonymity {
            tab.anonymize();
        }
        Ok(tab)
    }

    pub async fn load_from_rounds<C>(db: &C, round_ids: Vec<Uuid>, speaker_info: &super::info::TournamentParticipantsInfo) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
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

        let mut team_detailed_scores = speaker_info.teams_by_id.iter().map(|(k, _)| (*k, HashMap::new())).collect::<HashMap<_, _>>();
        let mut speaker_detailed_scores = speaker_info.speaker_teams.iter().map(|(k, _)| (*k, HashMap::new())).collect::<HashMap<_, _>>();
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
                        let speaker_team = speaker_info.speaker_teams.get(&speaker);
                        
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
                let (speaker_name, is_anonymous) = speaker_info.participants_by_id.get(&speaker_id).map(
                    |p| (p.name.clone(), p.is_anonymous)
                ).unwrap_or(("<Unknown Speaker>".to_string(), false));
                SpeakerTabEntry {
                    rank: 0,
                    speaker_name,
                    team_name: speaker_info.speaker_teams.get(&speaker_id).and_then(|t| speaker_info.teams_by_id.get(t)).map(|t| t.name.clone()).unwrap_or("<Unknown Team>".to_string()),
                    speaker_uuid: speaker_id,
                    total_score: per_round_score.values().map(|s| s.score).sum(),
                    avg_score: if per_round_score.values().len() > 0 {
                        Some(per_round_score.values().map(|s| s.score).sum::<f64>() /  per_round_score.values().len() as f64)
                    }
                    else {
                        None
                    },
                    detailed_scores: round_order.iter().map(|r| per_round_score.get(&r).cloned()).collect_vec(),
                    is_anonymous,
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
                    team_name: speaker_info.teams_by_id.get(&team_id).map(|t| t.name.clone()).unwrap_or("<Unknown Team>".to_string()),
                    team_uuid: team_id,
                    total_score: per_round_score.values().map(|s| s.total_score()).sum(),
                    avg_score: if per_round_score.values().len() > 0 {
                        Some(per_round_score.values().map(|s| s.total_score()).sum::<f64>() /  per_round_score.values().len() as f64)
                    }
                    else {
                        None
                    },
                    detailed_scores: round_order.iter().map(|r| per_round_score.get(&r).cloned()).collect_vec(),
                    member_ranks: speaker_info.team_members.get(&team_id).map(|members| {
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
        Ok(
            TabView { team_tab, speaker_tab, num_rounds: num_round_ids as u32 }
        )
    }

    pub async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        Self::load_from_tournament_with_anonymity(db, tournament_uuid, false).await
    }
    pub async fn load_from_tournament_with_anonymity<C>(db: &C, tournament_uuid: Uuid, respect_anonymity: bool) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(
            schema::tournament_round::Column::TournamentId.eq(tournament_uuid)
        ).all(db).await?;

        Self::load_from_tournament_with_rounds_with_anonymity(db, tournament_uuid, rounds.into_iter().map(|r| r.uuid).collect(), respect_anonymity).await
    }

    pub async fn load_from_tournament_with_rounds<C>(db: &C, tournament_uuid: Uuid, round_ids: Vec<Uuid>) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let speaker_info = super::info::TournamentParticipantsInfo::load(db, tournament_uuid).await?;
        Self::load_from_rounds(db, round_ids, &speaker_info).await
    }

    pub async fn load_from_tournament_with_rounds_with_anonymity<C>(db: &C, tournament_uuid: Uuid, round_ids: Vec<Uuid>, respect_anonymity: bool) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let speaker_info = super::info::TournamentParticipantsInfo::load(db, tournament_uuid).await?;
        let mut tab = Self::load_from_rounds(db, round_ids, &speaker_info).await?;

        if respect_anonymity {
            tab.anonymize();
        }
        Ok(tab)
    }

    fn detail_score_for_debate_side(ballot: &Ballot, team_role: &TeamRoundRole) -> (Option<f64>, Vec<f64>) {
        let (team_score, speaker_scores) = match team_role {
            TeamRoundRole::Government => (ballot.government.team_score(), ballot.government_speech_scores()),
            TeamRoundRole::Opposition => (ballot.opposition.team_score(), ballot.opposition_speech_scores()),
            TeamRoundRole::NonAligned => panic!("Can't compute team score for non-aligned speakers")
        };
        (team_score, speaker_scores)
    }

    fn add_scores_for_team(team_tab_entries: &mut VecMap<Uuid, TeamTabEntryDetailedScore>, round: &schema::tournament_round::Model, ballot: &Ballot, ballot_team: &BallotTeam, team_role: TeamRoundRole) {
        if let Some(team) = ballot_team.team {
            //let team_score = ballot_team.team_score();
            //let speaker_score = ballot.speeches.iter().filter(|s| s.role == SpeechRole::Government).map(|s| s.speaker_score()).sum::<f64>();
            let (total_score, speaker_scores) = match team_role {
                TeamRoundRole::Government => (ballot.government.team_score(), ballot.government_speech_scores()),
                TeamRoundRole::Opposition => (ballot.opposition.team_score(), ballot.opposition_speech_scores()),
                TeamRoundRole::NonAligned => panic!("Can't compute team score for non-aligned speakers")
            };

            if total_score.is_some() || speaker_scores.len() > 0 {
                team_tab_entries.insert(
                    &team,
                    round.index as usize,
                    TeamTabEntryDetailedScore { team_score: total_score, speaker_score: speaker_scores.into_iter().sum(), role: team_role }
                );
            }
        }
    }

    pub fn anonymize(&mut self) {
        for speaker in &mut self.speaker_tab {
            if speaker.is_anonymous {
                speaker.speaker_name = name_to_initials(&speaker.speaker_name);
            }
        }
    }

    pub fn anonymized(&self) -> Self {
        let mut cloned = self.clone();
        cloned.anonymize();
        cloned
    }
}


#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreakingAdjudicatorInfo {
    pub name: String,
    pub uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreakRelevantTabView {
    pub tab: TabView,
    pub speaker_teams: HashMap<Uuid, Uuid>,
    pub team_members: HashMap<Uuid, Vec<Uuid>>,
    pub breaking_teams: Vec<Uuid>,
    pub breaking_speakers: Vec<Uuid>,
    pub breaking_adjudicators: Vec<BreakingAdjudicatorInfo>
}

impl BreakRelevantTabView {
    pub async fn load_from_node<C>(db: &C, node_uuid: Uuid) -> Result<BreakRelevantTabView, anyhow::Error> where C: ConnectionTrait {
        Self::load_from_node_with_anonymity(db, node_uuid, false).await
    }
    
    pub async fn load_from_node_with_anonymity<C>(db: &C, node_uuid: Uuid, respect_anonymity: bool) -> Result<BreakRelevantTabView, anyhow::Error> where C: ConnectionTrait {
        let target_node = crate::domain::tournament_plan_node::TournamentPlanNode::get(db, node_uuid).await?;
        let break_background = BreakNodeBackgroundInfo::load_for_break_node(db, target_node.tournament_id, node_uuid).await?;
        let speaker_info = TournamentParticipantsInfo::load(db, target_node.tournament_id).await?;

        let break_id = match target_node.config {
            PlanNodeType::Break { break_id, .. } => {
                break_id
            },
            _ =>  None
        };

        let (breaking_teams, breaking_speakers, breaking_adjudicators) = match break_id {
            Some(break_id) => {
                let break_ = crate::domain::tournament_break::TournamentBreak::get(db, break_id).await?;
                let breaking_adjs = break_.breaking_adjudicators.into_iter().map(
                    |uuid| speaker_info.participants_by_id.get(&uuid).map(|p| BreakingAdjudicatorInfo {
                        name: if respect_anonymity {get_participant_public_name(p)} else {p.name.clone()},
                        uuid
                    }).unwrap_or_else(|| BreakingAdjudicatorInfo {
                        name: "<Unknown Adjudicator>".to_string(),
                        uuid
                    })).collect_vec();

                (break_.breaking_teams, break_.breaking_speakers, breaking_adjs)
            },
            None => (vec![], vec![], vec![])
        };

        let tab = TabView::load_from_rounds_with_anonymity(
            db,
            break_background.preceding_rounds.clone(),
            &speaker_info,
            respect_anonymity
        ).await?;

        Ok(BreakRelevantTabView {
            tab,
            speaker_teams: speaker_info.speaker_teams,
            team_members: speaker_info.team_members,
            breaking_teams,
            breaking_speakers,
            breaking_adjudicators
        })
    }
}
    */