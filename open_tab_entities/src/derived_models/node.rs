use sea_orm::prelude::*;
use std::collections::HashMap;
use thiserror::Error;

use crate::domain::{
    tournament_plan_node::{TournamentPlanNode, PlanNodeType},
    tournament_plan_edge::TournamentPlanEdge
};


pub struct BreakNodeBackgroundInfo {
    pub all_nodes: HashMap<Uuid, TournamentPlanNode>,
    pub preceding_rounds: Vec<Uuid>,
    pub relevant_break_id: Option<Option<Uuid>>
}

#[derive(Error, Debug)]
pub enum NodeExecutionError {
    #[error("Can only draw multiple rounds for standard preliminaries draw")]
    CanOnlyDrawMultipleRoundsForStandardPreliminariesDraw,
    #[error("Round is not in tournament {tournament_id}")]
    RoundIsNotInTournament { tournament_id: Uuid },
    #[error("Can not draw round without draw mode")]
    CanNotDrawRoundWithoutDrawMode,
    #[error("Missing break for round")]
    MissingBreak,
}

impl BreakNodeBackgroundInfo {
    fn new(all_nodes: HashMap<Uuid, TournamentPlanNode>, preceding_rounds: Vec<Uuid>, relevant_break_id: Option<Option<Uuid>>) -> Self {
        Self {
            all_nodes,
            preceding_rounds,
            relevant_break_id
        }
    }

    pub async fn load_for_break_node<C>(db: &C, tournament_id: Uuid, node_id: Uuid) -> Result<Self, anyhow::Error> where C: ConnectionTrait {
        let all_nodes = TournamentPlanNode::get_all_in_tournament(db, tournament_id).await?;
        let edges = TournamentPlanEdge::get_all_for_sources(db, all_nodes.iter().map(|n| n.uuid).collect()).await?;
        let all_nodes = all_nodes.into_iter().map(|n| (n.uuid, n)).collect::<HashMap<_, _>>();
        let parent_map = edges.into_iter().map(|e| (e.target_id, e.source_id)).collect::<HashMap<_, _>>();
        let mut curr_node_id = node_id;
        let mut relevant_break_id = None;
        let mut preceding_rounds = vec![];
        loop {
            let node = all_nodes.get(&curr_node_id).ok_or(NodeExecutionError::RoundIsNotInTournament { tournament_id })?;
            match &node.config {
                PlanNodeType::Break { config, break_id } => {
                    if relevant_break_id.is_none() {
                        relevant_break_id = Some(break_id.clone());
                    }
                },
                PlanNodeType::Round { config, rounds } => {
                    preceding_rounds.extend(rounds);
                }
            }
    
            let parent = parent_map.get(&curr_node_id);
            if let Some(parent) = parent {
                curr_node_id = *parent;
            } else {
                break;
            }
        };
        Ok(Self::new(all_nodes, preceding_rounds, relevant_break_id))
    }
}
