use std::{error::Error, collections::HashMap};

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, ActiveValue};
use serde::{Serialize, Deserialize};

use crate::schema;

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TournamentRound {
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub index: u64
}


impl TournamentRound {
    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<TournamentRound>, DbErr> where C: ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(schema::tournament_round::Column::Uuid.is_in(uuids)).all(db).await?;
        rounds.into_iter().map(|round| {
            Ok(TournamentRound {
                uuid: round.uuid,
                tournament_id: round.tournament_id,
                index: round.index as u64
            })
        }).collect()
    }

    pub fn new(tournament_id: Uuid, index: u64) -> Self {
        TournamentRound {
            uuid: Uuid::new_v4(),
            tournament_id,
            index
        }
    }
}

#[async_trait]
impl TournamentEntity for TournamentRound {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let model = schema::tournament_round::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            tournament_id: ActiveValue::Set(self.tournament_id),
            index: ActiveValue::Set(self.index as i32)
        };
        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let existing_model = schema::tournament_round::Entity::find().filter(schema::tournament_round::Column::Uuid.eq(self.uuid)).one(db).await?;
            if let Some(_) = existing_model {
                model.update(db).await?;
            }
            else {
                model.insert(db).await?;
            }
        };

        Ok(())
    }

    async fn get_many_tournaments<C>(_db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, Box<dyn Error>> where C: ConnectionTrait {
        Ok(entities.iter().map(|round| {
            Some(round.tournament_id)
        }).collect())
    }
}
