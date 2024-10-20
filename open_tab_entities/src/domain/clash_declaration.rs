use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::schema;
use crate::utilities::BatchLoadError;

use open_tab_macros::SimpleEntity;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, SimpleEntity, Default)]
#[module_path = "crate::schema::clash_declaration"]
pub struct ClashDeclaration {
    pub uuid: Uuid,
    pub was_seen: bool,
    pub source_participant_id: Uuid,
    pub target_participant_id: Uuid,
    pub severity: i32,
    pub is_retracted: bool,
}

impl ClashDeclaration {
    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<ClashDeclaration>, BatchLoadError> where C: ConnectionTrait {
        let clashes = schema::clash_declaration::Entity::find()
        .join(sea_orm::JoinType::InnerJoin, schema::clash_declaration::Relation::Participant2.def())
        .filter(schema::participant::Column::TournamentId.eq(tournament_id))
        .all(db)
        .await?;
        Ok(
            clashes.into_iter().map(
                ClashDeclaration::from_model
            ).collect()
        )
    }
}