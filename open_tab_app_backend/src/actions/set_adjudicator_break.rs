

use open_tab_entities::{EntityGroup, EntityGroupTrait, Entity, domain::entity::LoadEntity};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;

use open_tab_entities::domain::tournament_institution::TournamentInstitution;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetAdjudicatorBreakAction {
    pub node_id: Uuid,
    pub breaking_adjudicators: Vec<Uuid>
}


#[async_trait]
impl ActionTrait for SetAdjudicatorBreakAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new();

        let node = open_tab_entities::domain::tournament_plan_node::TournamentPlanNode::get(db, self.node_id).await?;

        match &node.config {
            open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round { config, rounds } => anyhow::bail!("Cannot set adjudicator break on round"),
            open_tab_entities::domain::tournament_plan_node::PlanNodeType::Break { config, break_id } => {
                if let Some(break_id) = break_id {
                    let mut break_ = open_tab_entities::domain::tournament_break::TournamentBreak::get(db, *break_id).await?;
                    break_.breaking_adjudicators = self.breaking_adjudicators;
                    g.add(Entity::TournamentBreak(break_));
                }
                else {
                    anyhow::bail!("Cannot set adjudicator break on break node without break id. Must make team break first.");
                }
            },
        }

        Ok(
            g
        )       
    }
}