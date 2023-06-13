use async_trait::async_trait;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, SimpleEntity)]
#[module_path = "crate::schema::tournament_venue"]
#[tournament_id = "tournament_id"]
pub struct TournamentVenue {
    pub uuid: Uuid,
    pub name: String,
    pub tournament_id: Uuid
}

impl TournamentVenue {
    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<TournamentVenue>, DbErr> where C: ConnectionTrait {
        let rows = schema::tournament_venue::Entity::find().filter(schema::tournament_venue::Column::TournamentId.eq(tournament_id)).all(db).await?;
        Ok(rows.into_iter().map(Self::from_model).collect())
    }
}
