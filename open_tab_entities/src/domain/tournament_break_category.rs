use open_tab_macros::SimpleEntity;
use sea_orm::schema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;
use sea_orm::entity::prelude::*;
use sea_orm::ActiveModelTrait;
use sea_orm::EntityTrait;
use sea_orm::ColumnTrait;
use sea_orm::QueryFilter;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, SimpleEntity)]
#[module_path = "crate::schema::tournament_break_category"]
#[tournament_id = "tournament_id"]
pub struct TournamentBreakCategory {
    pub uuid: Uuid,
    pub name: String,
    pub tournament_id: Uuid,
}

impl TournamentBreakCategory {
    pub fn new(name: String, tournament_id: Uuid) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            name,
            tournament_id,
        }
    }

    pub async fn get_all_in_tournament<C>(
        db: &C,
        tournament_id: Uuid,
    ) -> Result<Vec<TournamentBreakCategory>, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        let categories = crate::schema::tournament_break_category::Entity::find()
            .filter(crate::schema::tournament_break_category::Column::TournamentId.eq(tournament_id))
            .all(db)
            .await?;

        Ok(categories.into_iter().map(Self::from_model).collect())
    }
}