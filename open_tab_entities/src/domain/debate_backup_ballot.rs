use std::{error::Error, collections::HashMap};

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, ActiveValue};
use serde::{Serialize, Deserialize};

use crate::schema;

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct DebateBackupBallot {
    pub uuid: Uuid,
    pub debate_id: Uuid,
    pub ballot_id: Uuid,
    pub timestamp: DateTime
}


impl DebateBackupBallot {
    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Self>, DbErr> where C: ConnectionTrait {
        let debates = schema::debate_backup_ballot::Entity::find().filter(schema::debate_backup_ballot::Column::Uuid.is_in(uuids)).all(db).await?;
        debates.into_iter().map(|debate| {
            Ok(DebateBackupBallot {
                uuid: debate.uuid,
                debate_id: debate.debate_id,
                ballot_id: debate.ballot_id,
                timestamp: debate.timestamp,
            })
        }).collect()
    }
}

#[async_trait]
impl TournamentEntity for DebateBackupBallot {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let model = schema::debate_backup_ballot::ActiveModel {
            uuid: ActiveValue::Set(self.uuid),
            debate_id: ActiveValue::Set(self.debate_id),
            ballot_id: ActiveValue::Set(self.ballot_id),
            timestamp: ActiveValue::Set(self.timestamp)
        };
        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let existing_model = schema::debate_backup_ballot::Entity::find().filter(schema::debate_backup_ballot::Column::Uuid.eq(self.uuid)).one(db).await?;
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
        let debates_with_rounds = schema::tournament_debate::Entity::find()
            .filter(
                schema::tournament_debate::Column::Uuid.is_in(entities.iter().map(|backup| backup.debate_id).collect_vec()
            )
        ).find_with_related(schema::tournament_round::Entity).all(db).await?;

        let debate_tournaments = debates_with_rounds.into_iter().map(
            |(debate, rounds)| {
                let round = rounds.into_iter().next().expect("Presence of round is guaranteed by the schema");
                (debate.uuid, round.tournament_id)
            }
        ).collect::<HashMap<_, _>>();

        entities.iter().map(|b| {
            Ok(debate_tournaments.get(&b.debate_id).cloned())
        }).collect()
    }
}
