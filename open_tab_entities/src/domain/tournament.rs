use std::{error::Error, collections::HashMap};

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, ActiveValue};
use serde::{Serialize, Deserialize};

use crate::schema;

use crate::utilities::{BatchLoad, BatchLoadError};

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Tournament {
    pub uuid: Uuid,
}


impl Tournament {
    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Tournament>, BatchLoadError> where C: ConnectionTrait {
        let tournaments = schema::ballot::Entity::batch_load_all(db, uuids).await?;
        tournaments.into_iter().map(|tournament| {
            Ok(Tournament {
                uuid: tournament.uuid,
            })
        }).collect()
    }

    pub fn new() -> Self {
        Tournament {
            uuid: Uuid::new_v4(),
        }
    }
}

#[async_trait]
impl TournamentEntity for Tournament {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let model = schema::tournament::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
        };
        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let existing_model = schema::tournament::Entity::find().filter(schema::tournament::Column::Uuid.eq(self.uuid)).one(db).await?;
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
        return Ok(entities.iter().map(|tournament| {
            Some(tournament.uuid)
        }).collect());
    }
}
