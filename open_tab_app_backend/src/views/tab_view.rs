use std::fmt::Display;
use std::hash::Hash;
use std::iter::{zip, self};
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use open_tab_entities::domain::entity::LoadEntity;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::{prelude::*, domain};

use open_tab_entities::schema::{self};

use itertools::Itertools;

use ordered_float::OrderedFloat;

use super::base::LoadedView;


pub struct LoadedTabView {
    pub view: TabView,
    pub tournament_uuid: Uuid
}

impl LoadedTabView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<LoadedTabView, Box<dyn Error>> where C: ConnectionTrait {
        Ok(
            LoadedTabView {
                tournament_uuid,
                view: TabView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedTabView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.ballots.len() > 0 {
            self.view = TabView::load_from_tournament(db, self.tournament_uuid).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabView {
    pub team_tab: Vec<TeamTabEntry>,
    pub speaker_tab: Vec<SpeakerTabEntry>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTabEntry {
    pub team_name: String,
    pub team_uuid: Uuid,
    pub total_points: f64,
    pub avg_points: f64,
    pub detailed_scores: Vec<Option<TeamTabEntryDetailedScore>>
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeamRoundRole {
    Government,
    Opposition,
    NonAligned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTabEntry {
    pub speaker_name: String,
    pub speaker_uuid: Uuid,
    pub total_points: f64,
    pub avg_points: f64,
    pub detailed_scores: Vec<Option<SpeakerTabEntryDetailedScore>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTabEntryDetailedScore {
    pub score: f64,
    pub team_role: TeamRoundRole,
    pub speech_position: u8
}

#[derive(Debug)]
enum DrawViewError {
}

impl Display for DrawViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for DrawViewError {
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

    /*fn get(&self, key: &K, index: usize) -> Option<&V> {
        let vec = self.store.get(key)?;
        vec.get(index)?.as_ref()
    }*/

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
            self.store.insert(key.clone(), iter::repeat(None).take(index).collect());
            self.store.get_mut(key).unwrap()
        };
        
        if vec.len() <= index {
            vec.extend(iter::repeat(None).take(index - vec.len() + 1));
        }

        vec[index] = Some(value);
    }
}

impl TabView {
    pub async fn load_from_rounds<C>(db: &C, round_ids: Vec<Uuid>, speaker_info: &super::base::TournamentParticipantsInfo) -> Result<TabView, Box<dyn Error>> where C: ConnectionTrait {
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

        //let mut team_tab : HashMap<Uuid, Vec<_>> = HashMap::new();
        //let mut speaker_tab = HashMap::new();

        let num_rounds = rounds.len();
        let rounds_and_debates = zip(rounds.into_iter(), ballots.into_iter()).sorted_by_key(|(round, _)| round.index).collect_vec();
        
        let mut team_tab_entries : VecMap<Uuid, _> = VecMap::new();
        for team in speaker_info.teams_by_id.keys() {
            team_tab_entries.reserve(team, num_rounds);
        }

        let mut speaker_tab_entries = VecMap::new();
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
                                    team_tab_entries.insert(
                                        speaker_team,
                                        round.index as usize,
                                        TeamTabEntryDetailedScore {
                                            team_score: None,
                                            speaker_score: speech.speaker_score().unwrap_or(0.0),
                                            role: TeamRoundRole::NonAligned
                                        }
                                    );
                                }
                            }
                        }
                    }

                    speaker_tab_entries.insert(
                        &speaker,
                        round.index as usize,
                        SpeakerTabEntryDetailedScore {
                            score: speech.speaker_score().unwrap_or(0.0),
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

        let mut team_tab = vec![];
        for (team, total_score) in total_team_scores {
            let detailed_scores = team_tab_entries.store.get(&team).unwrap().clone();
            let num_rounds = detailed_scores.iter().filter(|s| s.is_none()).count();
            let team_tab_entry = TeamTabEntry {
                detailed_scores,
                team_name: speaker_info.teams_by_id.get(&team).unwrap().name.clone(),
                team_uuid: team,
                total_points: total_score,
                avg_points: total_score / num_rounds as f64,
            };

            team_tab.push(team_tab_entry);
        }

        let mut speaker_tab = vec![];
        for (speaker, total_score) in total_speaker_scores {
            let detailed_scores = speaker_tab_entries.store.get(&speaker).unwrap().clone();
            let num_rounds = detailed_scores.iter().filter(|s| s.is_none()).count();
            let speaker_tab_entry = SpeakerTabEntry {
                detailed_scores,
                speaker_name: speaker_info.participants_by_id.get(&speaker).unwrap().name.clone(),
                speaker_uuid: speaker,
                total_points: total_score,
                avg_points: total_score / num_rounds as f64,
            };

            speaker_tab.push(speaker_tab_entry);
        }

        Ok(
            TabView { team_tab, speaker_tab }
        )
    }

    pub async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<TabView, Box<dyn Error>> where C: ConnectionTrait {
        let speaker_info = super::base::TournamentParticipantsInfo::load(db, tournament_uuid).await?;
        let rounds = schema::tournament_round::Entity::find().filter(
            schema::tournament_round::Column::TournamentId.eq(tournament_uuid)
        ).all(db).await?;

        Self::load_from_rounds(db, rounds.into_iter().map(|r| r.uuid).collect(), &speaker_info).await
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

            team_tab_entries.insert(
                &team,
                round.index as usize,
                TeamTabEntryDetailedScore { team_score: total_score, speaker_score: speaker_scores.into_iter().sum(), role: team_role }
            );
        }
    }
}