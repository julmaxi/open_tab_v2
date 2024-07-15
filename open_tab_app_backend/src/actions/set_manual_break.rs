


use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{entity::LoadEntity, tournament_plan_node::{TournamentPlanNode, PlanNodeType}}};


use sea_orm::prelude::*;


use serde::{Serialize, Deserialize};
use open_tab_entities::domain::tournament_break::TournamentBreak;
use super::ActionTrait;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetManualBreakAction {
    node_id: Uuid,
    breaking_teams: Vec<Uuid>,
    breaking_speakers: Vec<Uuid>
}

#[async_trait]
impl ActionTrait for SetManualBreakAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut node = TournamentPlanNode::get(db, self.node_id).await?;
        let mut groups = EntityGroup::new(
            node.tournament_id
        );

        let tournament_id = node.tournament_id;

        let (prev_break_id, config) = match node.config {
            PlanNodeType::Break { config, break_id } => {
                (break_id, config)
            },
            _ => return Err(anyhow::anyhow!("Node is not a manual break"))
        };

        let break_id = prev_break_id.unwrap_or(Uuid::new_v4());

        let mut break_ = TournamentBreak::new(tournament_id);
        break_.uuid = break_id;
        break_.breaking_speakers = self.breaking_speakers;
        break_.breaking_teams = self.breaking_teams;

        node.config = PlanNodeType::Break {
            config,
            break_id: Some(break_id)
        };

        groups.add(Entity::TournamentPlanNode(node));
        groups.add(Entity::TournamentBreak(break_));

        Ok(groups)
    }
}