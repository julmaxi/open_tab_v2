use std::str::FromStr;


use async_trait::async_trait;
use itertools::{Itertools, izip};
use sea_orm::{prelude::*, ActiveValue, QueryOrder};
use serde::{Serialize, Deserialize};

use crate::schema;
use crate::utilities::BatchLoad;

use super::BoundTournamentEntityTrait;
use super::entity::{LoadEntity, TournamentEntityTrait};


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
pub struct TournamentBreak {
    pub uuid: Uuid,
    pub tournament_id: Uuid,

    pub breaking_teams: Vec<Uuid>,
    pub breaking_speakers: Vec<Uuid>,
    //Note that an empty list of adjudicators means
    //that there is no adjudicator break at all.
    //All adjudicators proceed to the next round.
    pub breaking_adjudicators: Vec<Uuid>,
}

impl TournamentBreak {
    pub fn new(tournament_id: Uuid) -> Self {
        TournamentBreak {
            uuid: Uuid::new_v4(),
            tournament_id,
            breaking_teams: vec![],
            breaking_speakers: vec![],
            breaking_adjudicators: vec![],
        }
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<Self>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let breaks = schema::tournament_break::Entity::find()
            .filter(
                schema::tournament_break::Column::TournamentId.eq(tournament_id)
            )
        .all(db).await?;

        let teams = breaks.load_many(schema::tournament_break_team::Entity, db).await?;
        let speakers = breaks.load_many(schema::tournament_break_speaker::Entity, db).await?;
        let adjudicators = breaks.load_many(schema::tournament_break_adjudicator::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            breaks,
            teams,
            speakers,
            adjudicators
        ).into_iter().map(|(break_row, teams, speakers, adjudicators)| {
            Self::from_rows(break_row, teams, speakers, adjudicators)
        }).collect();
        r
    }

    pub fn from_rows(
        break_row: schema::tournament_break::Model,
        teams: Vec<schema::tournament_break_team::Model>,
        speakers: Vec<schema::tournament_break_speaker::Model>,
        adjudicators: Vec<schema::tournament_break_adjudicator::Model>,
    ) -> Result<Self, anyhow::Error> {
        let breaking_teams = teams.into_iter().sorted_by_key(|team| team.position).map(|t| t.team_id).collect();
        let breaking_speakers = speakers.into_iter().sorted_by_key(|speaker| speaker.position).map(|s| s.speaker_id).collect();
        let breaking_adjudicators = adjudicators.into_iter().sorted_by_key(|a| a.adjudicator_id).map(|a| a.adjudicator_id).collect();

        Ok(Self {
            uuid: break_row.uuid,
            tournament_id: break_row.tournament_id,
            breaking_teams,
            breaking_speakers,
            breaking_adjudicators
        })
    }
}


#[async_trait]
impl LoadEntity for TournamentBreak {
    async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Self>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let breaks = schema::tournament_break::Entity::batch_load(db, uuids).await?;
        let exists_mask = breaks.iter().map(|b| b.is_some()).collect::<Vec<_>>();

        let breaks = breaks.into_iter().flatten().collect::<Vec<_>>();

        let teams = breaks.load_many(schema::tournament_break_team::Entity, db).await?;
        let speakers = breaks.load_many(schema::tournament_break_speaker::Entity, db).await?;
        let adjudicators = breaks.load_many(schema::tournament_break_adjudicator::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            breaks,
            teams,
            speakers,
            adjudicators
        ).into_iter().map(|(break_row, teams, speakers, adjudicators)| {
            Self::from_rows(break_row, teams, speakers, adjudicators)
        }).collect();
        r.map(|r| super::utils::pad(r, &exists_mask))
    }
}

#[async_trait]
impl<C> BoundTournamentEntityTrait<C> for TournamentBreak where C: sea_orm::ConnectionTrait {
    async fn save(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> {
        let model = schema::tournament_break::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            tournament_id: ActiveValue::Set(self.tournament_id),
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

        if guarantee_insert {
            if self.breaking_adjudicators.len() > 0 {
                schema::tournament_break_adjudicator::Entity::insert_many(self.breaking_adjudicators.iter().map(|a| {
                    schema::tournament_break_adjudicator::ActiveModel {
                        tournament_break_id: ActiveValue::Set(self.uuid),
                        adjudicator_id: ActiveValue::Set(*a),
                    }
                }).collect_vec()).exec(db).await?;    
            }
        } else {
            let prev_adjudicators = schema::tournament_break_adjudicator::Entity::find()
                .filter(schema::tournament_break_adjudicator::Column::TournamentBreakId.eq(self.uuid))
                .all(db)
                .await?;

            let to_delete = prev_adjudicators.iter().filter(|a| !self.breaking_adjudicators.contains(&a.adjudicator_id)).map(|a| a.adjudicator_id).collect_vec();
            let to_add = self.breaking_adjudicators.iter().filter(|a| !prev_adjudicators.iter().any(|p| p.adjudicator_id == **a)).collect_vec();

            if to_delete.len() > 0 {
                schema::tournament_break_adjudicator::Entity::delete_many().filter(
                    schema::tournament_break_adjudicator::Column::TournamentBreakId.eq(self.uuid)
                        .and(schema::tournament_break_adjudicator::Column::AdjudicatorId.is_in(to_delete))
                ).exec(db).await?;    
            }

            let to_insert = to_add.into_iter().map(|adj_id| {
                schema::tournament_break_adjudicator::ActiveModel {
                    tournament_break_id: ActiveValue::Set(self.uuid),
                    adjudicator_id: ActiveValue::Set(*adj_id),
                }
            }).collect_vec();
            if to_insert.len() > 0 {
                schema::tournament_break_adjudicator::Entity::insert_many(to_insert).exec(db).await?;
            }
        };

        Ok(())
    }

    async fn get_many_tournaments(_db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> {
        return Ok(entities.iter().map(|team| {
            Some(team.tournament_id)
        }).collect());
    }
    
    async fn delete_many(db: &C, ids: Vec<Uuid>) -> Result<(), anyhow::Error> {
        schema::tournament_break::Entity::delete_many().filter(schema::tournament_break::Column::Uuid.is_in(ids)).exec(db).await?;
        Ok(())
    }
}

impl TournamentEntityTrait for TournamentBreak {
    fn get_related_uuids(&self) -> Vec<Uuid> {
        let mut out = vec![self.tournament_id];
        out.extend(self.breaking_teams.iter());
        out.extend(self.breaking_speakers.iter());
        out.extend(self.breaking_adjudicators.iter());
        out
    }
}
