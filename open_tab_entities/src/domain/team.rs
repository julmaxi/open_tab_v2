

use async_trait::async_trait;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, SimpleEntity)]
#[module_path = "crate::schema::team"]
#[tournament_id = "tournament_id"]
pub struct Team {
    pub uuid: Uuid,
    pub name: String,
    pub tournament_id: Uuid
}

impl Team {
    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<Team>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let teams = schema::team::Entity::find().filter(schema::team::Column::TournamentId.eq(tournament_id)).all(db).await?;
        Ok(teams.into_iter().map(Self::from_model).collect())
    }
}
