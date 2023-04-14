use std::{error::Error, collections::HashMap};

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, ActiveValue};
use serde::{Serialize, Deserialize};

use crate::schema;
use crate::utilities::{BatchLoad, BatchLoadError};

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TournamentDebate {
    pub uuid: Uuid,
    pub round_id: Uuid,
    pub index: u64,
    pub current_ballot_uuid: Uuid
}


impl TournamentDebate {
    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<TournamentDebate>, BatchLoadError> where C: ConnectionTrait {
        let debates = schema::tournament_debate::Entity::batch_load_all(db, uuids).await?;
        debates.into_iter().map(|debate| {
            Ok(TournamentDebate {
                uuid: debate.uuid,
                round_id: debate.round_id,
                current_ballot_uuid: debate.ballot_id,
                index: debate.index as u64
            })
        }).collect()
    }

    pub async fn get_one<C>(db: &C, uuid: Uuid) -> Result<TournamentDebate, BatchLoadError> where C: ConnectionTrait {
        let debates = schema::tournament_debate::Entity::batch_load_all(db, vec![uuid]).await?;
        debates.into_iter().map(|debate| {
            Ok(TournamentDebate {
                uuid: debate.uuid,
                round_id: debate.round_id,
                current_ballot_uuid: debate.ballot_id,
                index: debate.index as u64
            })
        }).collect::<Result<Vec<TournamentDebate>, BatchLoadError>>().map(|mut debates| debates.pop().unwrap())
    }
}

#[async_trait]
impl TournamentEntity for TournamentDebate {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let model = schema::tournament_debate::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            ballot_id: ActiveValue::Set(self.current_ballot_uuid),
            round_id: ActiveValue::Set(self.round_id),
            index: ActiveValue::Set(self.index as i32)
        };
        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let existing_model = schema::tournament_debate::Entity::find().filter(schema::tournament_debate::Column::Uuid.eq(self.uuid)).one(db).await?;
            if let Some(_) = existing_model {
                model.update(db).await?;
            }
            else {
                model.insert(db).await?;
            }
        };

        Ok(())
    }

    async fn get_many_tournaments<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, Box<dyn Error>> where C: ConnectionTrait {
        let round_ids = entities.iter().map(|debate| debate.round_id).collect_vec();
        let mut rounds = schema::tournament_round::Entity::find().filter(schema::tournament_round::Column::Uuid.is_in(round_ids)).all(db).await?;

        /*let uuid_positions : HashMap<Uuid, usize> = entities.iter().enumerate().map(|(i, debate)| (debate.round_id, i)).collect();

        rounds.sort_by_key(|round| uuid_positions.get(&round.uuid).unwrap());*/

        Ok(rounds.into_iter().map(|round| {
            Some(round.tournament_id)
        }).collect())
    }
}
