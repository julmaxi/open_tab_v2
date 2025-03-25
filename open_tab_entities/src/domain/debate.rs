

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, QuerySelect};
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
    pub is_motion_released_to_non_aligned: bool,
    pub is_complete: bool
}


impl TournamentDebate {
    pub fn new(round_id: Uuid, index: u64, ballot_id: Uuid, venue_id: Option<Uuid>) -> Self {
        Self::new_with_uuid(Uuid::new_v4(), round_id, index, ballot_id, venue_id)
    }
    
    pub fn new_with_uuid(uuid: Uuid, round_id: Uuid, index: u64, ballot_id: Uuid, venue_id: Option<Uuid>) -> Self {
        Self {
            uuid,
            round_id,
            index,
            ballot_id,
            venue_id,
            is_motion_released_to_non_aligned: false,
            is_complete: false
        }
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> anyhow::Result<Vec<TournamentDebate>> where C: sea_orm::ConnectionTrait {
        Ok(
            schema::tournament_debate::Entity::find()
            .inner_join(schema::tournament_round::Entity)
            .filter(schema::tournament_round::Column::TournamentId.eq(tournament_id)).all(db).await?.into_iter().map(Self::from_model).collect_vec()
        )
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
