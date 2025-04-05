use open_tab_entities::{EntityGroup, Entity};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;

use open_tab_entities::domain::tournament_break_category::TournamentBreakCategory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBreakCategoryAction {
    pub name: String,
    pub uuid: Uuid,
    pub tournament_uuid: Uuid,
}

#[async_trait]
impl ActionTrait for CreateBreakCategoryAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, anyhow::Error> 
    where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new(self.tournament_uuid);
        g.add(
            Entity::TournamentBreakCategory(TournamentBreakCategory {
                uuid: self.uuid,
                name: self.name,
                tournament_id: self.tournament_uuid,
            })
        );
        Ok(g)
    }
}
