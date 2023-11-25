

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;
use crate::utilities::BatchLoadError;

use open_tab_macros::SimpleEntity;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, SimpleEntity, Default)]
#[module_path = "crate::schema::tournament_debate"]
#[get_many_tournaments_func = "get_many_tournaments_impl"]
pub struct TournamentDebate {
    pub uuid: Uuid,
    pub round_id: Uuid,
    pub index: u64,
    pub ballot_id: Uuid,
    pub venue_id: Option<Uuid>,
    pub is_motion_released_to_non_aligned: bool
}


impl TournamentDebate {
    async fn get_many_tournaments_impl<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let round_ids = entities.iter().map(|debate| debate.round_id).collect_vec();
        let rounds = schema::tournament_round::Entity::find().filter(schema::tournament_round::Column::Uuid.is_in(round_ids)).all(db).await?;

        Ok(rounds.into_iter().map(|round| {
            Some(round.tournament_id)
        }).collect())
    }

    pub async fn get_all_in_rounds<C>(db: &C, round_uuids: Vec<Uuid>) -> Result<Vec<Vec<TournamentDebate>>, BatchLoadError> where C: sea_orm::ConnectionTrait {
        let mut round_debates: Vec<Vec<TournamentDebate>> = vec![];
        for round_uuid in round_uuids {
            let debates = schema::tournament_debate::Entity::find().filter(schema::tournament_debate::Column::RoundId.eq(round_uuid)).all(db).await?;
            round_debates.push(debates.into_iter().map(Self::from_model).collect_vec())
        }
        Ok(round_debates)
    }
}
