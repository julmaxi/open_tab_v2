

use open_tab_entities::{EntityGroup, Entity};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;

use open_tab_entities::domain::tournament_institution::TournamentInstitution;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstitutionAction {
    pub name: String,
    pub uuid: Uuid,
    pub tournament_uuid: Uuid
}


#[async_trait]
impl ActionTrait for CreateInstitutionAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new(self.tournament_uuid);

        g.add(
            Entity::TournamentInstitution(TournamentInstitution {
                uuid: self.uuid,
                tournament_id: self.tournament_uuid,
                name: self.name,
            })
        );
        Ok(
            g
        )       
    }
}