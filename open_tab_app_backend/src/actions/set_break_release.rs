

use open_tab_entities::{EntityGroup, Entity, domain::{entity::LoadEntity, self}, derived_models::BreakNodeBackgroundInfo};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBreakReleaseAction {
    pub node_uuid: Uuid,
    pub time: chrono::NaiveDateTime
}


#[async_trait]
impl ActionTrait for SetBreakReleaseAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let node = open_tab_entities::domain::tournament_plan_node::TournamentPlanNode::get(db, self.node_uuid).await?;
        
        let mut g = EntityGroup::new(node.tournament_id);

        let break_background = BreakNodeBackgroundInfo::load_for_break_node(db, node.tournament_id, node.uuid).await?;

        let mut preceding_rounds = domain::round::TournamentRound::get_many(db, break_background.preceding_rounds).await?;
        for elem in preceding_rounds.iter_mut() {
            elem.feedback_release_time = Some(self.time);
            g.add(Entity::TournamentRound(elem.clone()));
        }
        Ok(
            g
        )       
    }
}