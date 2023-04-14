use std::{error::Error, collections::HashMap, fmt::Display};

use async_trait::async_trait;
use itertools::Itertools;
use sea_query::{Expr, SimpleExpr, SeaRc, ColumnRef};
use sea_query::Alias;
use sea_orm::{prelude::*, ActiveValue, QuerySelect, QueryTrait, DbBackend};
use serde::{Serialize, Deserialize};

use crate::{schema, utilities::{load_many, BatchLoadError, BatchLoad}};


use sea_orm::JoinType;

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct ParticipantClash {
    pub uuid: Uuid,
    pub declaring_participant_id: Uuid,
    pub target_participant_id: Uuid,
    pub severity: u16
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
                severity: clash.clash_severity as u16
            })
        }).collect())
    }

    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<ParticipantClash>, ParticipantClashParseError> where C: ConnectionTrait {
        let institutions = schema::participant_clash::Entity::batch_load_all(db, uuids.clone()).await.map_err(|e| match e {
            BatchLoadError::DbErr(e) => ParticipantClashParseError::DbErr(e),
            BatchLoadError::RowNotFound {..} => ParticipantClashParseError::ClashDoesNotExist
        })?;

        Ok(institutions.into_iter().map(|clash| {
            ParticipantClash {
                uuid: clash.uuid,
                declaring_participant_id: clash.declaring_participant_id,
                target_participant_id: clash.target_participant_id,
                severity: clash.clash_severity as u16
            }
        }).collect())
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<Self>, DbErr> where C: ConnectionTrait {
        let p1_alias = Alias::new("p1");
        let p2_alias = Alias::new("p2");

        let rows = schema::participant_clash::Entity::find()
            .join_as(
                JoinType::InnerJoin,
                schema::participant_clash::Relation::Participant1.def(),
                p1_alias.clone()
            )
            .join_as(
                JoinType::InnerJoin,
                schema::participant_clash::Relation::Participant2.def(),
                p2_alias.clone()
            )
            .filter(
                SimpleExpr::Column(
                    ColumnRef::TableColumn(
                        SeaRc::new(p1_alias),
                        SeaRc::new(schema::participant::Column::TournamentId)
                    )
                ).eq(tournament_id).and(
                    SimpleExpr::Column(
                        ColumnRef::TableColumn(
                            SeaRc::new(p2_alias),
                            SeaRc::new(schema::participant::Column::TournamentId)
                        )
                    ).eq(tournament_id)
                )
            ).all(db).await?;
        Ok(rows.into_iter().map(Self::from_row).collect())
    }

    fn from_row(row: schema::participant_clash::Model) -> Self {
        Self {
            uuid: row.uuid,
            declaring_participant_id: row.declaring_participant_id,
            target_participant_id: row.target_participant_id,
            severity: row.clash_severity as u16
        }
    }
}

#[async_trait]
impl TournamentEntity for ParticipantClash {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let model = schema::participant_clash::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            declaring_participant_id: ActiveValue::Set(self.declaring_participant_id),
            target_participant_id: ActiveValue::Set(self.target_participant_id),
            clash_severity: ActiveValue::Set(self.severity as i16),
        };
        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let existing_model = schema::participant_clash::Entity::find().filter(schema::participant_clash::Column::Uuid.eq(self.uuid)).one(db).await?;
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
        let participants = schema::participant::Entity::find()
            .filter(schema::participant::Column::Uuid.is_in(entities.iter().map(|entity| entity.uuid).collect_vec()))
            .all(db)
            .await?;

        let tournament_uuids = participants.into_iter().map(|p| Some(p.tournament_id)).collect_vec();
        Ok(tournament_uuids)
    }
}
