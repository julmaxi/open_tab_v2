use std::{error::Error, collections::HashMap};

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, ActiveValue};
use serde::{Serialize, Deserialize};

use crate::schema;

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Team {
    pub uuid: Uuid,
    pub name: String,
    pub tournament_id: Uuid
}


impl Team {
    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Team>, DbErr> where C: ConnectionTrait {
        let teams = schema::team::Entity::find().filter(schema::team::Column::Uuid.is_in(uuids)).all(db).await?;
        teams.into_iter().map(|team| {
            Ok(Team {
                uuid: team.uuid,
                name: team.name,
                tournament_id: team.tournament_id,
            })
        }).collect()
    }
}

#[async_trait]
impl TournamentEntity for Team {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let model = schema::team::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            name: ActiveValue::Set(self.name.clone()),
            tournament_id: ActiveValue::Set(self.tournament_id),
        };
        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let existing_model = schema::team::Entity::find().filter(schema::team::Column::Uuid.eq(self.uuid)).one(db).await?;
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
        return Ok(entities.iter().map(|team| {
            Some(team.tournament_id)
        }).collect());
    }
}
