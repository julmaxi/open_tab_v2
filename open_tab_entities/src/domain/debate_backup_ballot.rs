use std::collections::HashMap;

use async_trait::async_trait;
use itertools::Itertools;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, SimpleEntity)]
#[module_path = "crate::schema::debate_backup_ballot"]
#[get_many_tournaments_func = "get_many_tournaments_impl"]
pub struct DebateBackupBallot {
    pub uuid: Uuid,
    pub debate_id: Uuid,
    pub ballot_id: Uuid,
    pub timestamp: DateTime
}


impl DebateBackupBallot {
    async fn get_many_tournaments_impl<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
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
