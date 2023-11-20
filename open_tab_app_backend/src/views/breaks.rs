use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use open_tab_entities::{tab::TabView, domain::{entity::LoadEntity, tournament_plan_node::PlanNodeType, self}, info::TournamentParticipantsInfo, EntityGroup};
use sea_orm::{prelude::Uuid, ConnectionTrait};
use serde::Serialize;

use crate::{views, LoadedView, tournament_tree_view::get_round_names};



pub struct LoadedBreaksView {
    tournament_uuid: Uuid,
    view: BreaksView
}

impl LoadedBreaksView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: ConnectionTrait {
        Ok(
            Self {
                tournament_uuid,
                view: BreaksView::load(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedBreaksView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.tournament_breaks.len() > 0 {
            self.view = BreaksView::load(db, self.tournament_uuid).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreaksView {
    breaks: Vec<BreakInfo>
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BreakInfo {
    node_id: Uuid,
    break_id: Uuid,
    name: String,
}


impl BreaksView {
    async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: ConnectionTrait {
        //let rounds = domain::round::TournamentRound::get_all_in_tournament(db, tournament_uuid).await?;
        let breaks = domain::tournament_break::TournamentBreak::get_all_in_tournament(db, tournament_uuid).await?;
        let nodes = domain::tournament_plan_node::TournamentPlanNode::get_all_in_tournament(db, tournament_uuid).await?;
        let edges = domain::tournament_plan_edge::TournamentPlanEdge::get_all_for_sources(db, nodes.iter().map(|n| n.uuid).collect_vec()).await?;

        let nodes_to_parents = edges.iter().map(
            |edge| {
                (edge.target_id, edge.source_id)
            }
        ).collect::<HashMap<_, _>>();

        let node_children = edges.iter().map(
            |edge| {
                (edge.source_id, edge.target_id)
            }
        ).into_group_map();

        let round_names = get_round_names(nodes.clone(), &node_children, &nodes.iter().filter_map(
            |n| {
                if nodes_to_parents.contains_key(&n.uuid) {
                    None
                }
                else {
                    Some(n.uuid)
                }
            }
        ).collect::<Vec<_>>())?;

        let mut breaks = vec![];

        for node in nodes.iter() {
            match &node.config {
                PlanNodeType::Break { break_id: Some(break_id), .. } => {
                    let empty = vec![];
                    let break_node_children = node_children.get(&node.uuid).unwrap_or(&empty);
                    
                    let break_name = match break_node_children.len() {
                        0 => "Break to Nowhere".to_string(),
                        1 => round_names.get(&(break_node_children[0], 0)).unwrap_or(&"Unknown round".to_string()).clone(),
                        _ => "Break to Multiple Rounds".to_string() // This should never happen in normal use, but it could with special configuration
                    };

                    breaks.push(
                        BreakInfo {
                            node_id: node.uuid,
                            break_id: *break_id,
                            name: break_name,
                        }
                    )
                },
                _ => {}
            }
        }

        Ok(BreaksView {
            breaks
        })
    }
}