use std::str::FromStr;
use std::error::Error;

use async_trait::async_trait;
use itertools::{Itertools, izip};
use sea_orm::{prelude::*, ActiveValue, QueryOrder};
use serde::{Serialize, Deserialize};

use crate::schema;
use crate::utilities::{BatchLoad};

use super::TournamentEntity;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum BreakType {
    TabBreak{num_debates: u16},
    TwoThirdsBreak,
    KOBreak,
    TimBreak
}

impl FromStr for BreakType {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl ToString for BreakType {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}


impl BreakType {
    pub fn human_readable_description(&self) -> String {
        match self {
            BreakType::TabBreak{num_debates} => format!("Top {0} break", num_debates * 2),
            BreakType::TwoThirdsBreak => "Upper 2/3rds break".to_string(),
            BreakType::KOBreak => "Debate winners break".to_string(),
            BreakType::TimBreak => "Upper 1/3rd breaks, along with non-aligned".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum TournamentBreakSourceRoundType {
    Tab,
    Knockout,
}

impl ToString for TournamentBreakSourceRoundType {
    fn to_string(&self) -> String {
        match self {
            TournamentBreakSourceRoundType::Tab => "Tab".to_string(),
            TournamentBreakSourceRoundType::Knockout => "Knockout".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TournamentBreakSourceRound {
    pub break_type: TournamentBreakSourceRoundType,
    pub uuid: Uuid,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TournamentBreak {
    pub uuid: Uuid,
    pub source_rounds: Vec<TournamentBreakSourceRound>,
    pub child_rounds: Vec<Uuid>,
    pub tournament_id: Uuid,
    pub break_type: BreakType,

    pub breaking_teams: Vec<Uuid>,
    pub breaking_speakers: Vec<Uuid>,
}


impl TournamentBreak {
    pub fn new(tournament_id: Uuid, break_type: BreakType) -> Self {
        TournamentBreak {
            uuid: Uuid::new_v4(),
            source_rounds: vec![],
            child_rounds: vec![],
            tournament_id,
            break_type,
            breaking_teams: vec![],
            breaking_speakers: vec![],
        }
    }

    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Self>, Box<dyn Error>> where C: ConnectionTrait {
        let breaks = schema::tournament_break::Entity::batch_load_all(db, uuids).await?;
        let source_rounds = breaks.load_many(schema::tournament_break_source_round::Entity, db).await?;
        let child_rounds = breaks.load_many(schema::tournament_break_child_round::Entity, db).await?;
        let teams = breaks.load_many(schema::tournament_break_team::Entity, db).await?;
        let speakers = breaks.load_many(schema::tournament_break_speaker::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            breaks,
            source_rounds,
            child_rounds,
            teams,
            speakers
        ).into_iter().map(|(break_row, source_rounds, child_rounds, teams, speakers)| {
            Self::from_rows(break_row, source_rounds, child_rounds, teams, speakers)
        }).collect();
        r
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<Self>, Box<dyn Error>> where C: ConnectionTrait {
        let breaks = schema::tournament_break::Entity::find()
            .filter(
                schema::tournament_break::Column::TournamentId.eq(tournament_id)
            )
        .all(db).await?;

        let source_rounds = breaks.load_many(schema::tournament_break_source_round::Entity, db).await?;
        let child_rounds = breaks.load_many(schema::tournament_break_child_round::Entity, db).await?;
        let teams = breaks.load_many(schema::tournament_break_team::Entity, db).await?;
        let speakers = breaks.load_many(schema::tournament_break_speaker::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            breaks,
            source_rounds,
            child_rounds,
            teams,
            speakers
        ).into_iter().map(|(break_row, source_rounds, child_rounds, teams, speakers)| {
            Self::from_rows(break_row, source_rounds, child_rounds, teams, speakers)
        }).collect();
        r
    }

    pub fn from_rows(
        break_row: schema::tournament_break::Model,
        source_rounds: Vec<schema::tournament_break_source_round::Model>,
        child_rounds: Vec<schema::tournament_break_child_round::Model>,
        teams: Vec<schema::tournament_break_team::Model>,
        speakers: Vec<schema::tournament_break_speaker::Model>,
    ) -> Result<Self, Box<dyn Error>> {
        let breaking_teams = teams.into_iter().sorted_by_key(|team| team.position).map(|t| t.team_id).collect();
        let breaking_speakers = speakers.into_iter().sorted_by_key(|speaker| speaker.position).map(|s| s.speaker_id).collect();
        let source_rounds : Result<_, _> = source_rounds.into_iter().sorted_by_key(
            |r| r.tournament_round_id
        ).map(
            |r| {
                let break_type = match r.dependency_type.as_str() {
                    "Tab" => Ok::<_, &'static str>(TournamentBreakSourceRoundType::Tab),
                    "Knockout" => Ok(TournamentBreakSourceRoundType::Knockout),
                    _ => return Err("Invalid break type"),
                }?;
                Ok(TournamentBreakSourceRound {
                    break_type,
                    uuid: r.tournament_round_id,
                })
            }
        ).collect();
        let child_rounds = child_rounds.into_iter().map(|r| r.tournament_round_id).sorted().collect();

        let break_type : Result<BreakType, _> = break_row.break_type.parse();

        Ok(Self {
            uuid: break_row.uuid,
            source_rounds: source_rounds?,
            child_rounds,
            tournament_id: break_row.tournament_id,
            break_type: break_type?,
            breaking_teams,
            breaking_speakers,
        })
    }
}

#[async_trait]
impl TournamentEntity for TournamentBreak {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let model = schema::tournament_break::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            tournament_id: ActiveValue::Set(self.tournament_id),
            break_type: ActiveValue::Set(self.break_type.to_string()),
        };

        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let prev_model = schema::tournament_break::Entity::find_by_id(self.uuid).one(db).await?;

            if let Some(_) = prev_model {
                model.update(db).await?;
            } else {
                model.insert(db).await?;
            }
        }

        let num_required_teams = self.breaking_teams.len();
        if guarantee_insert {
            if num_required_teams > 0 {
                schema::tournament_break_team::Entity::insert_many((0..num_required_teams).map(|i| {
                    schema::tournament_break_team::ActiveModel {
                        tournament_break_id: ActiveValue::Set(self.uuid),
                        team_id: ActiveValue::Set(self.breaking_teams[i]),
                        position: ActiveValue::Set(i as i32),
                    }
                }).collect_vec()).exec(db).await?;    
            }
        } else {
            let prev_teams = schema::tournament_break_team::Entity::find()
                .filter(schema::tournament_break_team::Column::TournamentBreakId.eq(self.uuid))
                .order_by_asc(schema::tournament_break_team::Column::Position)
                .all(db)
                .await?;

            let teams_to_keep = prev_teams.iter().take(num_required_teams).collect_vec();

            for (i, team) in teams_to_keep.iter().enumerate() {
                let model = schema::tournament_break_team::ActiveModel {
                    tournament_break_id: ActiveValue::Set(self.uuid),
                    team_id: ActiveValue::Set(self.breaking_teams[i]),
                    position: ActiveValue::Set(i as i32),
                };

                if team.team_id != self.breaking_teams[i] {
                    model.update(db).await?;
                }
            }

            if num_required_teams < prev_teams.len() {
                schema::tournament_break_team::Entity::delete_many().filter(
                    schema::tournament_break_team::Column::TournamentBreakId.eq(self.uuid)
                        .and(schema::tournament_break_team::Column::Position.gte(num_required_teams as i32))
                ).exec(db).await?;
            }
            else if num_required_teams > prev_teams.len() {
                let to_insert = (prev_teams.len()..num_required_teams).map(|i| {
                    schema::tournament_break_team::ActiveModel {
                        tournament_break_id: ActiveValue::Set(self.uuid),
                        team_id: ActiveValue::Set(self.breaking_teams[i]),
                        position: ActiveValue::Set(i as i32),
                    }
                }).collect_vec();

                schema::tournament_break_team::Entity::insert_many(to_insert).exec(db).await?;
            }
        };

        let num_required_speakers = self.breaking_speakers.len();
        if guarantee_insert {
            if num_required_speakers > 0 {
                schema::tournament_break_speaker::Entity::insert_many((0..num_required_speakers).map(|i| {
                    schema::tournament_break_speaker::ActiveModel {
                        tournament_break_id: ActiveValue::Set(self.uuid),
                        speaker_id: ActiveValue::Set(self.breaking_speakers[i]),
                        position: ActiveValue::Set(i as i32),
                    }
                }).collect_vec()).exec(db).await?;    
            }
        } else {
            let prev_speakers = schema::tournament_break_speaker::Entity::find()
                .filter(schema::tournament_break_speaker::Column::TournamentBreakId.eq(self.uuid))
                .order_by_asc(schema::tournament_break_speaker::Column::Position)
                .all(db)
                .await?;

            let speakers_to_keep = prev_speakers.iter().take(num_required_speakers).collect_vec();

            for (i, speaker) in speakers_to_keep.iter().enumerate() {
                let model = schema::tournament_break_speaker::ActiveModel {
                    tournament_break_id: ActiveValue::Set(self.uuid),
                    speaker_id: ActiveValue::Set(self.breaking_speakers[i]),
                    position: ActiveValue::Set(i as i32),
                };

                if speaker.speaker_id != self.breaking_speakers[i] {
                    model.update(db).await?;
                }
            }

            if num_required_speakers < prev_speakers.len() {
                schema::tournament_break_speaker::Entity::delete_many().filter(
                    schema::tournament_break_speaker::Column::TournamentBreakId.eq(self.uuid)
                        .and(schema::tournament_break_speaker::Column::Position.gte(num_required_speakers as i32))
                ).exec(db).await?;
            }
            else if num_required_speakers > prev_speakers.len() {
                let to_insert = (prev_speakers.len()..num_required_speakers).map(|i| {
                    schema::tournament_break_speaker::ActiveModel {
                        tournament_break_id: ActiveValue::Set(self.uuid),
                        speaker_id: ActiveValue::Set(self.breaking_speakers[i]),
                        position: ActiveValue::Set(i as i32),
                    }
                }).collect_vec();

                schema::tournament_break_speaker::Entity::insert_many(to_insert).exec(db).await?;
            }
        };

        let new_child_rounds = self.child_rounds.iter().map(|round_id| {
            schema::tournament_break_child_round::ActiveModel {
                tournament_break_id: ActiveValue::Set(self.uuid),
                tournament_round_id: ActiveValue::Set(*round_id),
            }
        }).collect_vec();

        if guarantee_insert {
            if !new_child_rounds.is_empty() {
                schema::tournament_break_child_round::Entity::insert_many(new_child_rounds).exec(db).await?;
            }
        } else {
            let prev_child_rounds = schema::tournament_break_child_round::Entity::find()
                .filter(schema::tournament_break_child_round::Column::TournamentBreakId.eq(self.uuid))
                .all(db)
                .await?;

            let to_delete = prev_child_rounds.iter().filter_map(|prev_child_round| {
                let round_uuid = prev_child_round.tournament_round_id;
                if !self.child_rounds.contains(&round_uuid) {
                    Some(round_uuid)
                } else {
                    None
                }
            }).collect_vec();

            if !to_delete.is_empty() {
                schema::tournament_break_child_round::Entity::delete_many().filter(
                    schema::tournament_break_child_round::Column::TournamentBreakId.eq(self.uuid)
                        .and(schema::tournament_break_child_round::Column::TournamentRoundId.is_in(to_delete))
                ).exec(db).await?;
            }

            let to_insert = new_child_rounds.iter().filter_map(|new_child_round| {
                let round_uuid = new_child_round.tournament_round_id.clone();
                if !prev_child_rounds.iter().any(|prev_child_round| prev_child_round.tournament_round_id == *round_uuid.as_ref()) {
                    Some(new_child_round.clone())
                } else {
                    None
                }
            }).collect_vec();

            if !to_insert.is_empty() {
                schema::tournament_break_child_round::Entity::insert_many(to_insert).exec(db).await?;
            }
        }

        let new_source_rounds = self.source_rounds.iter().map(|dependency| {
            schema::tournament_break_source_round::ActiveModel {
                tournament_break_id: ActiveValue::Set(self.uuid),
                tournament_round_id: ActiveValue::Set(dependency.uuid),
                dependency_type: ActiveValue::Set(dependency.break_type.to_string())
            }
        }).collect_vec();

        if guarantee_insert {
            if new_source_rounds.len() > 0 {
                schema::tournament_break_source_round::Entity::insert_many(new_source_rounds).exec(db).await?;
            }
        } else {
            let prev_source_rounds = schema::tournament_break_source_round::Entity::find()
                .filter(schema::tournament_break_source_round::Column::TournamentBreakId.eq(self.uuid))
                .all(db)
                .await?;

            let source_uuids = self.source_rounds.iter().map(|dependency| {
                dependency.uuid
            }).collect_vec();
            let to_delete = prev_source_rounds.iter().filter_map(|prev_source_round| {
                let round_uuid = prev_source_round.tournament_round_id;
                if !source_uuids.contains(&round_uuid) {
                    Some(round_uuid)
                } else {
                    None
                }
            }).collect_vec();

            if !to_delete.is_empty() {
                schema::tournament_break_source_round::Entity::delete_many().filter(
                    schema::tournament_break_source_round::Column::TournamentBreakId.eq(self.uuid)
                        .and(schema::tournament_break_source_round::Column::TournamentRoundId.is_in(to_delete))
                ).exec(db).await?;
            }

            let to_insert = new_source_rounds.iter().filter_map(|new_source_round| {
                let round_uuid = new_source_round.tournament_round_id.clone();
                if !prev_source_rounds.iter().any(|prev_source_round| prev_source_round.tournament_round_id == *round_uuid.as_ref()) {
                    Some(new_source_round.clone())
                } else {
                    None
                }
            }).collect_vec();

            if !to_insert.is_empty() {
                schema::tournament_break_source_round::Entity::insert_many(to_insert).exec(db).await?;
            }
        }

        Ok(())
    }

    async fn get_many_tournaments<C>(_db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, Box<dyn Error>> where C: ConnectionTrait {
        return Ok(entities.iter().map(|team| {
            Some(team.tournament_id)
        }).collect());
    }
}
