use async_trait::async_trait;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, SimpleEntity)]
#[module_path = "crate::schema::tournament_institution"]
#[tournament_id = "tournament_id"]
pub struct TournamentInstitution {
    pub uuid: Uuid,
    pub name: String,
    pub tournament_id: Uuid
}

impl TournamentInstitution {
    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<TournamentInstitution>, DbErr> where C: sea_orm::ConnectionTrait {
        let rows = schema::tournament_institution::Entity::find().filter(schema::tournament_institution::Column::TournamentId.eq(tournament_id)).all(db).await?;
        Ok(rows.into_iter().map(Self::from_model).collect())
    }
}
