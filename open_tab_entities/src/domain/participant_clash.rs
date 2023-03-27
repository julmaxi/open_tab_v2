use std::{error::Error, collections::HashMap, fmt::Display};

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, ActiveValue};
use serde::{Serialize, Deserialize};

use crate::{schema, utilities::{load_many, BatchLoadError, BatchLoad}};

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct ParticipantClash {
    pub uuid: Uuid,
    pub declaring_participant_id: Uuid,
    pub target_participant_id: Uuid,
    pub clash_strength: i16
}


#[derive(Debug)]
pub enum ParticipantClashParseError {
    DbErr(DbErr),
    ClashDoesNotExist
}

impl Display for ParticipantClashParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))?;
        Ok(())
    }
}

impl Error for ParticipantClashParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParticipantClashParseError::DbErr(e) => Some(e),
            _ => None
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl From<DbErr> for ParticipantClashParseError {
    fn from(value: DbErr) -> Self {
        ParticipantClashParseError::DbErr(value)
    }
}


impl ParticipantClash {
    pub async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<ParticipantClash>>, ParticipantClashParseError> where C: ConnectionTrait {
        let clashes = schema::participant_clash::Entity::batch_load(db, uuids).await?;

        Ok(clashes.into_iter().map(|clashes| {
            clashes.map(|clash| ParticipantClash {
                uuid: clash.uuid,
                declaring_participant_id: clash.declaring_participant_id,
                target_participant_id: clash.target_participant_id,
                clash_strength: clash.clash_strength
            })
        }).collect())
    }

    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<ParticipantClash>, ParticipantClashParseError> where C: ConnectionTrait {
        let institutions = schema::participant_clash::Entity::batch_load_all(db, uuids.clone()).await.map_err(|e| match e {
            BatchLoadError::DbErr(e) => ParticipantClashParseError::DbErr(e),
            BatchLoadError::RowNotFound => ParticipantClashParseError::ClashDoesNotExist
        })?;

        Ok(institutions.into_iter().map(|clash| {
            ParticipantClash {
                uuid: clash.uuid,
                declaring_participant_id: clash.declaring_participant_id,
                target_participant_id: clash.target_participant_id,
                clash_strength: clash.clash_strength
            }
        }).collect())
    }
}

#[async_trait]
impl TournamentEntity for ParticipantClash {
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
