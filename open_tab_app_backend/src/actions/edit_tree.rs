use std::{error::Error, collections::{HashMap, self}};

use itertools::Itertools;
use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{round::{self}, tournament_plan_edge::TournamentPlanEdge, tournament_plan_node::{TournamentPlanNode, PlanNodeConfig, BreakConfig, FoldDrawConfig}, self}, EntityType, group, schema::tournament_round};

use sea_orm::prelude::*;

use serde::{Serialize, Deserialize};
use thiserror::Error;

use super::ActionTrait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTreeAction {
    tournament_id: Uuid,
    action: EditTreeActionType
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EditTreeActionType {
    AddPreliminaryRounds { parent: Option<Uuid> },
    AddMinorBreakRounds { parent: Uuid, draws: Vec<FoldDrawConfig> },
    AddTimBreakRounds { parent: Uuid },
    AddKOStage { parent: Uuid, num_stages: u64 },
    UpdateNode { node: Uuid, config: PlanNodeConfig },
}

#[derive(Error, Debug)]
pub enum EditTreeActionError {
    #[error("the parent round does not exist")]
    ParentRoundDoesNotExist {uuid: Uuid},
}


pub fn reindex_rounds(
    all_nodes: &Vec<TournamentPlanNode>,
    all_edges: &Vec<TournamentPlanEdge>,
    all_rounds: &Vec<TournamentRound>
) -> Vec<TournamentRound> {
    let mut changed_rounds = vec![];

    let all_rounds = all_rounds.into_iter().map(|r| (r.uuid, r)).collect::<HashMap<Uuid, _>>();

    let node_parents = all_edges.iter().map(|e| (e.target_id, e.source_id)).collect::<HashMap<Uuid, Uuid>>();
    let roots = all_nodes.iter().filter(|n| !node_parents.contains_key(&n.uuid)).map(|n| n.uuid).collect::<Vec<Uuid>>();
    let node_children = all_edges.iter().map(|e| (e.source_id, e.target_id)).into_group_map();

    let mut explore_queue = vec![];

    explore_queue.extend(roots);

    let mut visited = collections::HashSet::new();

    let mut curr_round_index = 0;

    while let Some(next) = explore_queue.pop() {
        if visited.contains(&next) {
            continue;
        }
        visited.insert(next);

        let node = all_nodes.iter().find(|n| n.uuid == next).unwrap();
        match &node.config {
            open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round { config, rounds } => {
                let num_to_label = usize::max(config.num_rounds() as usize, rounds.len());
                for (local_idx, round_) in rounds.iter().enumerate() {
                    let mut round = all_rounds.get(round_).expect("Guaranteed by db constraints").clone().clone();
                    if round.index != curr_round_index {
                        round.index = curr_round_index + local_idx as u64;
                        changed_rounds.push(round);
                    }
                }

                curr_round_index += num_to_label as u64;
            },
            _ => {},
        }

        explore_queue.extend(node_children.get(&next).unwrap_or(&vec![]));
    }

    changed_rounds
}

#[async_trait]
impl ActionTrait for EditTreeAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut groups = EntityGroup::new();

        //let all_existing_rounds = TournamentRound::get_all_in_tournament(db, self.tournament_id).await?;

        let mut all_nodes = TournamentPlanNode::get_all_in_tournament(db, self.tournament_id).await?;
        let mut all_edges = TournamentPlanEdge::get_all_for_sources(db, all_nodes.iter().map(|n| n.uuid).collect()).await?;

        match self.action {
            EditTreeActionType::AddPreliminaryRounds { parent: parent_node } => {
                let node = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::Preliminaries { num_roundtrips: 1 },
                    rounds: vec![]
                });
                if let Some(parent_node) = parent_node {
                    let current_edges = TournamentPlanEdge::get_all_for_sources(db, vec![parent_node]).await?;
                    if let Some(first_child) = current_edges.first() {
                        groups.delete(EntityType::TournamentPlanEdge, first_child.uuid);
                        let edge = all_edges.iter_mut().find_position(|e| e.uuid == first_child.uuid).unwrap().0;
                        all_edges.remove(edge);
                        groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(node.uuid, first_child.target_id)));
                    }

                    groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(parent_node, node.uuid)));
                }

                groups.add(Entity::TournamentPlanNode(node));
            },         
            EditTreeActionType::AddKOStage { parent, num_stages } => {
                let mut nodes = vec![];
                let mut edges = vec![];
                let first_break = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::TabBreak { num_debates: u32::pow(2, (num_stages - 1) as u32) as u32 }));
                let first_round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        round_configs: vec![open_tab_entities::domain::tournament_plan_node::FoldDrawConfig::default_ko_fold()]
                    },
                    rounds: vec![]
                });

                let first_break_id = first_break.uuid;
                let first_round_id = first_round.uuid;

                nodes.push(first_break);
                nodes.push(first_round);

                edges.push(TournamentPlanEdge::new(parent, first_break_id));
                edges.push(TournamentPlanEdge::new(first_break_id, first_round_id));

                let mut last_id = first_round_id;

                for stage_idx in 1..num_stages {
                    let break_ = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::KnockoutBreak));
                    let round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                        config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                            round_configs: vec![open_tab_entities::domain::tournament_plan_node::FoldDrawConfig::default_ko_fold()]
                        },
                        rounds: vec![]
                    });

                    edges.push(TournamentPlanEdge::new(last_id, break_.uuid));
                    edges.push(TournamentPlanEdge::new(break_.uuid, round.uuid));

                    last_id = round.uuid;

                    nodes.push(break_);
                    nodes.push(round);
                }

                edges.into_iter().for_each(|e| groups.add(Entity::TournamentPlanEdge(e)));
                nodes.into_iter().for_each(|n| groups.add(Entity::TournamentPlanNode(n)));                
            },
            EditTreeActionType::AddMinorBreakRounds { parent, draws } => {
                let mut nodes: Vec<TournamentPlanNode> = vec![];
                let mut edges = vec![];

                let break_ = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::TwoThirdsBreak));                
                let break_id = break_.uuid;
                nodes.push(break_);

                edges.push(TournamentPlanEdge::new(parent, break_id));

                let node = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw{round_configs: draws},
                    rounds: vec![]
                });

                edges.push(TournamentPlanEdge::new(break_id, node.uuid));

                edges.into_iter().for_each(|e| groups.add(Entity::TournamentPlanEdge(e)));
                nodes.into_iter().for_each(|n| groups.add(Entity::TournamentPlanNode(n)));

            },
            EditTreeActionType::AddTimBreakRounds { parent } => {
                let first_break = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::TwoThirdsBreak));
                let first_round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        round_configs: vec![open_tab_entities::domain::tournament_plan_node::FoldDrawConfig {
                            team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::Random,
                            non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                            team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random,
                        }],
                    },
                    rounds: vec![]
                });

                let second_break = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::TimBreak));
                let second_round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        round_configs: vec![open_tab_entities::domain::tournament_plan_node::FoldDrawConfig {
                            team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::PowerPaired,
                            non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                            team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random,
                        }],
                    },
                    rounds: vec![]
                });

                groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(parent, first_break.uuid)));
                groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(first_break.uuid, first_round.uuid)));
                groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(first_round.uuid, second_break.uuid)));
                groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(second_break.uuid, second_round.uuid)));

                groups.add(Entity::TournamentPlanNode(first_break));
                groups.add(Entity::TournamentPlanNode(first_round));
                groups.add(Entity::TournamentPlanNode(second_break));
                groups.add(Entity::TournamentPlanNode(second_round));
            },
            EditTreeActionType::UpdateNode { node, config } => {
                //TODO:  Validation?
                let node = all_nodes.iter_mut().find(|n| n.uuid == node).unwrap();
                match (&mut node.config, config) {
                    (domain::tournament_plan_node::PlanNodeType::Round { config, .. }, PlanNodeConfig::RoundGroup { config: new_config }) => {
                        *config = new_config
                    },
                    (domain::tournament_plan_node::PlanNodeType::Break { config, .. }, PlanNodeConfig::Break { config: new_config }) => {
                        *config = new_config
                    },
                    _ => return Err(anyhow::anyhow!("Invalid node type for update"))
                }
                groups.add(Entity::TournamentPlanNode(node.clone()));
            }
        }

        all_edges.extend(groups.tournament_plan_edges.iter().map(|e| e.clone()));
        all_nodes.extend(groups.tournament_plan_nodes.iter().map(|n| n.clone()));

        let all_rounds = domain::round::TournamentRound::get_all_in_tournament(
            db,
            self.tournament_id
        ).await?;
    
        reindex_rounds(&all_nodes, &all_edges, &all_rounds).into_iter().for_each(|r| groups.add(Entity::TournamentRound(r)));

        Ok(groups)
    }
}