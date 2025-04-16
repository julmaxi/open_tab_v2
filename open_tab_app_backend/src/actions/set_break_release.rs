

use open_tab_entities::{derived_models::BreakNodeBackgroundInfo, domain::{self, entity::LoadEntity, tournament_break::TournamentBreak, tournament_plan_node::{PlanNodeType, TournamentPlanNode}}, prelude::TournamentRound, Entity, EntityGroup};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBreakReleaseAction {
    #[serde(default)]
    pub node_uuid: Option<Uuid>,
    #[serde(default)]
    pub tournament_uuid: Option<Uuid>,
    pub time: chrono::NaiveDateTime
}


#[async_trait]
impl ActionTrait for SetBreakReleaseAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let (mut g, mut preceding_rounds, mut breaks) = if let Some(node_uuid) = self.node_uuid {
            let node = TournamentPlanNode::get(db, node_uuid).await?;
            let g = EntityGroup::new(node.tournament_id);
            
            let break_background = BreakNodeBackgroundInfo::load_for_break_node(db, node.tournament_id, node.uuid).await?;

            let preceding_rounds = domain::round::TournamentRound::get_many(db, break_background.preceding_rounds).await?;

            let break_ = if let PlanNodeType::Break { break_id: Some(break_id), .. } = &node.config {
                let break_ = domain::tournament_break::TournamentBreak::get(db, *break_id).await?;
                vec![break_]
            }
            else {
                vec![]
            };
            (g, preceding_rounds, break_)
        }
        else if let Some(tournament_id) = self.tournament_uuid {
            let g = EntityGroup::new(tournament_id);
            let rounds = TournamentRound::get_all_in_tournament(db, tournament_id).await?;
            let breaks_ = TournamentBreak::get_all_in_tournament(db, tournament_id).await?;
            (
                g,
                rounds,
                breaks_
            )
        }
        else {
            return Err(anyhow::anyhow!("Must provide either a node_uuid or a tournament_uuid"));
        };

        for elem in preceding_rounds.iter_mut() {
            if elem.feedback_release_time.is_none() {
                elem.feedback_release_time = Some(self.time);
            }
            if elem.silent_round_results_release_time.is_none() {
                elem.silent_round_results_release_time = Some(self.time);
            }
            g.add(Entity::TournamentRound(elem.clone()));
        }
        for elem in breaks.iter_mut() {
            if elem.release_time.is_none() {
                elem.release_time = Some(self.time);
            }
            g.add(Entity::TournamentBreak(elem.clone()));
        }

        Ok(
            g
        )       
    }
}