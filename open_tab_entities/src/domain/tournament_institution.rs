use std::{error::Error, collections::HashMap, fmt::Display};

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, ActiveValue};
use serde::{Serialize, Deserialize};

use crate::{schema, utilities::{load_many, BatchLoadError, BatchLoad}};

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct TournamentInstitution {
    pub uuid: Uuid,
    pub name: String
}


#[derive(Debug)]
pub enum TournamentInstitutionParseError {
    DbErr(DbErr),
    InstitutionDoesNotExist
}

impl Display for TournamentInstitutionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))?;
        Ok(())
    }
}

impl Error for TournamentInstitutionParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TournamentInstitutionParseError::DbErr(e) => Some(e),
            _ => None
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl From<DbErr> for TournamentInstitutionParseError {
    fn from(value: DbErr) -> Self {
        TournamentInstitutionParseError::DbErr(value)
    }
}


impl TournamentInstitution {
    pub async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<TournamentInstitution>>, TournamentInstitutionParseError> where C: ConnectionTrait {
        let institutions = schema::tournament_institution::Entity::batch_load(db, uuids).await?;

        Ok(institutions.into_iter().map(|institution| {
            institution.map(|institution| TournamentInstitution {
                uuid: institution.uuid,
                name: institution.name
            })
        }).collect())
    }

    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<TournamentInstitution>, TournamentInstitutionParseError> where C: ConnectionTrait {
        let institutions = schema::tournament_institution::Entity::batch_load_all(db, uuids.clone()).await.map_err(|e| match e {
            BatchLoadError::DbErr(e) => TournamentInstitutionParseError::DbErr(e),
            BatchLoadError::RowNotFound => TournamentInstitutionParseError::InstitutionDoesNotExist
        })?;

        Ok(institutions.into_iter().map(|institution| {
            TournamentInstitution {
                uuid: institution.uuid,
                name: institution.name
            }
        }).collect())
    }
}

#[async_trait]
impl TournamentEntity for TournamentInstitution {
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
