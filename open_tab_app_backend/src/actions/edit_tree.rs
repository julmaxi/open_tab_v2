use std::error::Error;

use itertools::Itertools;
use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{round::{DrawType, TabDrawConfig, TeamAssignmentRule, self}, tournament_break::{TournamentBreak, TournamentBreakSourceRound, TournamentBreakSourceRoundType}, tournament_plan_edge::TournamentPlanEdge, tournament_plan_node::{TournamentPlanNode, PlanNodeConfig, BreakConfig, FoldDrawConfig}}, EntityType};

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
}

#[derive(Error, Debug)]
pub enum EditTreeActionError {
    #[error("the parent round does not exist")]
    ParentRoundDoesNotExist {uuid: Uuid},
}

#[async_trait]
impl ActionTrait for EditTreeAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut groups = EntityGroup::new();

        //let all_existing_rounds = TournamentRound::get_all_in_tournament(db, self.tournament_id).await?;

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
                        groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(node.uuid, first_child.target_id)));
                    }

                    groups.add(Entity::TournamentPlanEdge(TournamentPlanEdge::new(parent_node, node.uuid)));
                }

                groups.add(Entity::TournamentPlanNode(node));
            },
            
            EditTreeActionType::AddKOStage { parent, num_stages } => {
                let mut nodes = vec![];
                let mut edges = vec![];
                let num_breaking_teams = 1 << num_stages;
                let first_break = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::TabBreak { num_breaking_teams }));
                let first_round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        config: open_tab_entities::domain::tournament_plan_node::FoldDrawConfig::default_ko_fold()
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
                            config: open_tab_entities::domain::tournament_plan_node::FoldDrawConfig::default_ko_fold()
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

                let mut prev_node_id = break_id;

                for setting in draws {
                    let round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                        config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw{config: setting},
                        rounds: vec![]
                    });

                    edges.push(TournamentPlanEdge::new(prev_node_id, round.uuid));
                    prev_node_id = round.uuid;

                    nodes.push(round);
                }

                edges.into_iter().for_each(|e| groups.add(Entity::TournamentPlanEdge(e)));
                nodes.into_iter().for_each(|n| groups.add(Entity::TournamentPlanNode(n)));

            },
            EditTreeActionType::AddTimBreakRounds { parent } => {
                let first_break = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::TwoThirdsBreak));
                let first_round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        config: open_tab_entities::domain::tournament_plan_node::FoldDrawConfig {
                            team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::Random,
                            non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                            team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random,
                        },
                    },
                    rounds: vec![]
                });

                let second_break = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::new_break(BreakConfig::TimBreak));
                let second_round = TournamentPlanNode::new(self.tournament_id, open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round {
                    config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        config: open_tab_entities::domain::tournament_plan_node::FoldDrawConfig {
                            team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::PowerPaired,
                            non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                            team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random,
                        },
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
        }

        Ok(groups)
    }
}