
use std::fmt::Display;
use std::hash::Hash;
use std::iter::{zip, self, empty};
use std::{collections::HashMap, error::Error};

use async_trait::async_trait;
use crate::derived_models::BreakNodeBackgroundInfo;
use crate::domain::entity::LoadEntity;
use crate::domain::tournament_plan_node::PlanNodeType;
use crate::info::TournamentParticipantsInfo;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use crate::{prelude::*, domain};

use crate::schema::{self};

use itertools::Itertools;

use ordered_float::OrderedFloat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabView {
    pub num_rounds: u32,
    pub team_tab: Vec<TeamTabEntry>,
    pub speaker_tab: Vec<SpeakerTabEntry>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTabEntry {
    pub rank: u32,
    pub team_name: String,
    pub team_uuid: Uuid,
    pub total_score: f64,
    pub avg_score: Option<f64>,
    pub detailed_scores: Vec<Option<TeamTabEntryDetailedScore>>,
    //Be careful here: member ranks start at 1, not 0 for
    //convenience in the frontend
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
    pub speaker_name: String,
    pub team_name: String,
    pub speaker_uuid: Uuid,
    pub total_score: f64,
    pub avg_score: Option<f64>,
    pub detailed_scores: Vec<Option<SpeakerTabEntryDetailedScore>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTabEntryDetailedScore {
    pub score: f64,
    pub team_role: TeamRoundRole,
    pub speech_position: u8
}

struct VecMap<K, V> {
    store: HashMap<K, Vec<Option<V>>>
}

impl<K, V> VecMap<K, V> where K: Eq + Hash + Clone, V: Clone {
    fn new() -> VecMap<K, V> {
        VecMap {
            store: HashMap::new()
        }
    }

    fn get(&self, key: &K, index: usize) -> Option<&V> {
        let vec = self.store.get(key)?;
        vec.get(index)?.as_ref()
    }

    fn get_mut(&mut self, key: &K, index: usize) -> Option<&mut V> {
        let vec = self.store.get_mut(key)?;
        if let Some(val) = vec.get_mut(index) {
            val.as_mut()
        }
        else {
            None
        }
    }

    fn reserve(&mut self, key: &K, len: usize) {
        if let Some(vec) = self.store.get_mut(key) {
            if vec.len() < len {
                vec.extend(iter::repeat(None).take(len - vec.len() + 1));
            }
        }
        else {
            self.store.insert(key.clone(), iter::repeat(None).take(len).collect());
        }
    }

    fn insert(&mut self, key: &K, index: usize, value: V) {
        let vec = self.store.get_mut(key);
        let vec = if let Some(vec) = vec {
            vec
        }
        else {
            self.store.insert(key.clone(), vec![]);
            self.store.get_mut(key).unwrap()
        };
        
        if vec.len() <= index {
            vec.extend(iter::repeat(None).take(index - vec.len() + 1));
        }

        vec[index] = Some(value);
    }
}
impl TabView {
    pub async fn load_from_rounds<C>(db: &C, round_ids: Vec<Uuid>, speaker_info: &super::info::TournamentParticipantsInfo) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let num_round_ids = round_ids.len();
        let relevant_ballots = schema::tournament_debate::Entity::find()
        .inner_join(schema::tournament_round::Entity)
        .filter(schema::tournament_round::Column::Uuid.is_in(round_ids))
        .find_with_related(schema::ballot::Entity)
            .all(db)
            .await?;

        let rounds = relevant_ballots.iter().map(
            |(debate, _)| debate.clone()
        )
            .collect_vec()
            .load_one(schema::tournament_round::Entity, db).await?
            .into_iter()
            .map(
                |r|r.expect("DB constraint should prevent debate without round")
            ).collect_vec();
        let ballots = relevant_ballots.into_iter().map(|(_, mut ballots)| ballots.pop().expect("Schema should ensure there always is one ballot").uuid).collect_vec();

        let ballots = domain::ballot::Ballot::get_many(db, ballots).await?;
        let num_rounds = num_round_ids;
        let rounds_and_debates = zip(rounds.into_iter(), ballots.into_iter()).sorted_by_key(|(round, _)| round.index).collect_vec();
        
        let mut team_tab_entries : VecMap<Uuid, _> = VecMap::new();
        for team in speaker_info.teams_by_id.keys() {
            team_tab_entries.reserve(team, num_rounds);
        }

        let mut speaker_tab_entries: VecMap<Uuid, SpeakerTabEntryDetailedScore> = VecMap::new();
        for speaker in speaker_info.speaker_teams.keys() {
            speaker_tab_entries.reserve(speaker, num_rounds);
        }

        for (round, ballot) in rounds_and_debates {
            Self::add_scores_for_team(&mut team_tab_entries, &round, &ballot, &ballot.government, TeamRoundRole::Government);
            Self::add_scores_for_team(&mut team_tab_entries, &round, &ballot, &ballot.opposition, TeamRoundRole::Opposition);

            for speech in ballot.speeches {
                if let Some(speaker) = speech.speaker {
                    let speaker_team = speaker_info.speaker_teams.get(&speaker).unwrap();
                    let team_entry: Option<&mut TeamTabEntryDetailedScore> = team_tab_entries.get_mut(&speaker_team, round.index as usize);
                
                    match speech.role {
                        SpeechRole::Government | SpeechRole::Opposition => assert!(team_entry.is_some(), "Team entry should exist"),
                        SpeechRole::NonAligned => {
                            match team_entry {
                                Some(team_entry) => {
                                    team_entry.speaker_score += speech.speaker_score().unwrap_or(0.0);
                                },
                                None => {
                                    if let Some(speaker_score) = speech.speaker_score() {
                                        team_tab_entries.insert(
                                            speaker_team,
                                            round.index as usize,
                                            TeamTabEntryDetailedScore {
                                                team_score: None,
                                                speaker_score,
                                                role: TeamRoundRole::NonAligned
                                            }
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if let Some(score) = speech.speaker_score() {
                        if let Some(prev_score) = speaker_tab_entries.get(
                            &speaker,
                            round.index as usize
                        ) {
                            // Handle the super clevery thought out opt-out rule
                            if prev_score.score > score {
                                continue;
                            }
                        }
                        speaker_tab_entries.insert(
                            &speaker,
                            round.index as usize,
                            SpeakerTabEntryDetailedScore {
                                score,
                                team_role: match speech.role {
                                    SpeechRole::Government => TeamRoundRole::Government,
                                    SpeechRole::Opposition => TeamRoundRole::Opposition,
                                    SpeechRole::NonAligned => TeamRoundRole::NonAligned
                                },
                                speech_position: speech.position
                            }
                        );
                    }
                }
            }
        }

        let mut total_team_scores = team_tab_entries.store.iter().map(|(team, scores)| {
            let total_score = scores.iter().filter_map(|s| s.as_ref()).map(|s| s.total_score()).sum::<f64>();
            (*team, total_score)
        }).collect_vec();
        total_team_scores.sort_by_key(|t| -OrderedFloat(t.1));

        let mut total_speaker_scores = speaker_tab_entries.store.iter().map(|(speaker, scores)| {
            let total_score = scores.iter().filter_map(|s| s.as_ref()).map(|s| s.score).sum::<f64>();
            (*speaker, total_score)
        }).collect_vec();
        total_speaker_scores.sort_by_key(|s| -OrderedFloat(s.1));

        let mut speaker_tab = vec![];
        let mut prev_score = None;
        let mut rank = 0;
        let mut team_member_ranks = HashMap::new();
        for (idx, (speaker, total_score)) in total_speaker_scores.into_iter().enumerate() {
            if Some(total_score) != prev_score {
                rank = idx as u32;
            }
            let team_name = if let Some(team_id) = speaker_info.speaker_teams.get(&speaker) {
                team_member_ranks.entry(team_id).or_insert_with(|| vec![]).push(rank + 1);
                speaker_info.teams_by_id.get(&team_id).map(|t| t.name.clone()).unwrap_or("<Unknown Team>".to_string())
            }
            else {
                "<No Team>".to_string()
            };

            prev_score = Some(total_score);
            let detailed_scores = speaker_tab_entries.store.get(&speaker).unwrap().clone();
            let num_rounds = detailed_scores.iter().filter(|s| s.is_some()).count();
            let speaker_tab_entry = SpeakerTabEntry {
                rank,
                detailed_scores,
                speaker_name: speaker_info.participants_by_id.get(&speaker).unwrap().name.clone(),
                speaker_uuid: speaker,
                team_name,
                total_score,
                avg_score: if num_rounds > 0 { Some(total_score / num_rounds as f64) } else { None },
            };

            speaker_tab.push(speaker_tab_entry);
        }

        let mut team_tab = vec![];
        let mut prev_score = None;
        let mut rank = 0;
        for (idx, (team, total_score)) in total_team_scores.into_iter().enumerate() {
            if prev_score.is_some() && Some(total_score) != prev_score {
                rank = idx as u32;
            }
            prev_score = Some(total_score);
            let detailed_scores = team_tab_entries.store.get(&team).unwrap().clone();
            let num_rounds = detailed_scores.iter().filter(|s| s.is_some()).count();
            let team_tab_entry = TeamTabEntry {
                rank,
                detailed_scores,
                team_name: speaker_info.teams_by_id.get(&team).unwrap().name.clone(),
                team_uuid: team,
                total_score,
                avg_score: if num_rounds > 0 { Some(total_score / num_rounds as f64) } else { None },
                member_ranks: team_member_ranks.get(&team).cloned().unwrap_or(vec![])
            };

            team_tab.push(team_tab_entry);
        }

        Ok(
            TabView { team_tab, speaker_tab, num_rounds: num_round_ids as u32 }
        )
    }

    pub async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(
            schema::tournament_round::Column::TournamentId.eq(tournament_uuid)
        ).all(db).await?;

        Self::load_from_tournament_with_rounds(db, tournament_uuid, rounds.into_iter().map(|r| r.uuid).collect()).await
    }

    pub async fn load_from_tournament_with_rounds<C>(db: &C, tournament_uuid: Uuid, round_ids: Vec<Uuid>) -> Result<TabView, anyhow::Error> where C: ConnectionTrait {
        let speaker_info = super::info::TournamentParticipantsInfo::load(db, tournament_uuid).await?;
        Self::load_from_rounds(db, round_ids, &speaker_info).await
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
}



#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreakRelevantTabView {
    pub tab: TabView,
    pub speaker_teams: HashMap<Uuid, Uuid>,
    pub team_members: HashMap<Uuid, Vec<Uuid>>,
    pub breaking_teams: Vec<Uuid>,
    pub breaking_speakers: Vec<Uuid>
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

        let (breaking_teams, breaking_speakers) = match break_id {
            Some(break_id) => {
                let break_ = crate::domain::tournament_break::TournamentBreak::get(db, break_id).await?;

                (break_.breaking_teams, break_.breaking_speakers)
            },
            None => (vec![], vec![])
        };

        let tab = TabView::load_from_rounds(
            db,
            break_background.preceding_rounds.clone(),
            &speaker_info
        ).await?;

        Ok(BreakRelevantTabView {
            tab,
            speaker_teams: speaker_info.speaker_teams,
            team_members: speaker_info.team_members,
            breaking_teams,
            breaking_speakers: breaking_speakers
        })
    }
}