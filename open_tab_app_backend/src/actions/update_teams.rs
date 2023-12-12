

use open_tab_entities::{EntityGroup, EntityGroupTrait, Entity, domain::team::Team};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTeamsAction {
    pub updates: Vec<TeamUpdateRequest>,
    pub tournament_id: Uuid
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct TeamUpdateRequest {
    uuid: Uuid,
    name: String,
}


#[async_trait]
impl ActionTrait for UpdateTeamsAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new();

        for request in self.updates {
            g.add(
                Entity::Team(Team {
                    uuid: request.uuid,
                    name: request.name,
                    tournament_id: self.tournament_id
                })
            );
        }

        Ok(
            g
        )       
    }
}