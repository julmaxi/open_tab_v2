use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{prelude::*, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::schema;
use crate::utilities::BatchLoadError;

use open_tab_macros::SimpleEntity;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, SimpleEntity, Default)]
#[module_path = "crate::schema::institution_declaration"]
pub struct InstitutionDeclaration {
    pub uuid: Uuid,
    pub was_seen: bool,
    pub source_participant_id: Uuid,
    pub tournament_institution_id: Uuid,
    pub severity: i32,
    pub is_retracted: bool,
}

impl InstitutionDeclaration {
    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<InstitutionDeclaration>, BatchLoadError> where C: ConnectionTrait {
        let clashes = schema::institution_declaration::Entity::find()
        .join(sea_orm::JoinType::InnerJoin, schema::institution_declaration::Relation::Participant.def())
        .filter(schema::participant::Column::TournamentId.eq(tournament_id))
        .all(db)
        .await?;
        Ok(
            clashes.into_iter().map(
                InstitutionDeclaration::from_model
            ).collect()
        )
    }
}