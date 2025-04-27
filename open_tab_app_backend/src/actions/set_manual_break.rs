


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

        if let PlanNodeType::Break {
            break_id,
            suggested_break_award_prestige,
            suggested_award_title,
            suggested_award_series_key,
            ..
        } = &mut node.config {
            let mut break_ = if let Some(break_id) = break_id {
                TournamentBreak::get(db, *break_id).await?
            }
            else {
                TournamentBreak::new(tournament_id)
            };
            *break_id = Some(break_.uuid);
            
            break_.breaking_speakers = self.breaking_speakers;
            break_.breaking_teams = self.breaking_teams;

            break_.break_award_title = suggested_award_title.clone();
            break_.award_series_key = suggested_award_series_key.clone();
            break_.break_award_prestige = suggested_break_award_prestige.clone();
 
            groups.add(Entity::TournamentPlanNode(node));
            groups.add(Entity::TournamentBreak(break_));
    
            Ok(groups)
        }
        else {
            Err(anyhow::anyhow!("Node is not a break"))
        }
    }
}