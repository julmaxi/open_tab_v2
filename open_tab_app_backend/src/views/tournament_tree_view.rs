
use open_tab_entities::domain::tournament_plan_node::RoundGroupConfig;
use open_tab_entities::domain::tournament_plan_node::{TournamentPlanNode, BreakConfig, FoldDrawConfig};
use sea_orm::prelude::Uuid;


use std::collections::HashSet;

use std::vec;
use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;

use open_tab_entities::domain;


use itertools::Itertools;

use crate::{LoadedView, EditTreeActionType};

pub struct LoadedTournamentTreeView {
    pub view: TournamentTreeView,
    pub tournament_id: Uuid
}

impl LoadedTournamentTreeView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            LoadedTournamentTreeView {
                tournament_id: tournament_uuid,
                view: TournamentTreeView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedTournamentTreeView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.tournament_rounds.len() > 0 || changes.tournament_breaks.len() > 0 || changes.tournament_plan_nodes.len() > 0 || changes.tournament_plan_edges.len() > 0 {
            self.view = TournamentTreeView::load_from_tournament(db, self.tournament_id).await?;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TournamentTreeNode {
    uuid: Option<Uuid>,
    content: TournamentTreeNodeContent,
    children: Vec<Box<TournamentTreeNode>>,
    available_actions: Vec<AvailableAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AvailableAction {
    description: String,
    action: EditTreeActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoundInfo {
    uuid: Option<Uuid>,
    name: String,
    #[serde(flatten)]
    plan_state: RoundInfoState
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="plan_state")]
enum RoundInfoState {
    Ok,
    Ghost,
    Superflous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BreakInfo {
    uuid: Option<Uuid>,
    break_description: String,
    config: BreakConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoundGroupInfo {
    rounds: Vec<RoundInfo>,
    config: RoundGroupConfig
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum TournamentTreeNodeContent {
    Root,
    RoundGroup(RoundGroupInfo),
    Break(BreakInfo),
    Error
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentTreeView {
    tree: TournamentTreeNode,
}


fn is_pow2(n: i32) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

fn num_teams_to_round_name(num_teams: i32) -> String {
    match num_teams {
        2 => "Finals".into(),
        4 => "Semi-Finals".into(),
        8 => "Quarter-Finals".into(),
        16 => "Octo-Finals".into(),
        32 => "Double-Octo-Finals".into(),
        _ if is_pow2(num_teams) => format!("1/{} Break", num_teams / 2),
        _ => format!("{}-Team Break", num_teams),
    }
}

pub fn get_special_name_from_preceding_breaks(breaks: &Vec<&BreakConfig>) -> Option<String> {
    let most_recent = breaks.last();

    if breaks.is_empty() {
        return None;
    }
    let most_recent = most_recent.unwrap();

    match most_recent {
        BreakConfig::TabBreak { num_debates } => if is_pow2(*num_debates as i32) && *num_debates > 0 {
            Some(num_teams_to_round_name((num_debates * 2) as i32))
        }
        else {
            None
        },
        BreakConfig::KnockoutBreak => {
            let remaining_teams = get_num_remaining_teams_from_breaks(breaks);
            if let Some(remaining_teams) = remaining_teams {
                Some(num_teams_to_round_name(remaining_teams))
            }
            else {
                None
            }
        },
        _ => None
    }
}


fn get_num_remaining_teams_from_breaks(breaks: &Vec<&BreakConfig>) -> Option<i32> {
    if breaks.len() == 0 {
        return None;
    }

    let mut num_remaining = None;

    for break_ in breaks {
        num_remaining = match break_ {
            BreakConfig::Manual => None,
            BreakConfig::TabBreak { num_debates } => Some((num_debates * 2) as i32),
            BreakConfig::KnockoutBreak => if let Some(remaining) = num_remaining {
                if remaining % 2 == 0 {
                    Some(remaining / 2)
                }
                else {
                    return None;
                }
            }
            else {
                None
            },
            BreakConfig::TwoThirdsBreak => if let Some(remaining) = num_remaining {
                if remaining % 3 == 0 {
                    Some(remaining * 2 / 3)
                }
                else {
                    return None;
                }
            }
            else {
                None
            },
            BreakConfig::TimBreak => {
                if let Some(remaining) = num_remaining{
                    if remaining % 2 == 0 {
                        Some(remaining / 2)
                    }
                    else {
                        return None;
                    }
                }
                else {
                    None
                }
            },
        }
    }

    num_remaining
}

pub fn get_round_names(nodes: Vec<TournamentPlanNode>, node_children: &HashMap<Uuid, Vec<Uuid>>, roots: &Vec<Uuid>) -> Result<HashMap<(Uuid, usize), String>, anyhow::Error> {
    let mut explore_queue = roots.clone().into_iter().map(|r| (r, vec![])).collect_vec();

    if explore_queue.len() == 0 && nodes.len() > 0 {
        return Err(anyhow::anyhow!("Tournament plan is not a tree"));
    }

    let mut names = HashMap::new();
    let mut visited = HashSet::new();

    let mut curr_idx = 0;

    let empty_vec = vec![];

    while explore_queue.len() > 0 {
        let (next_node_id, prev_breaks) = explore_queue.pop().unwrap();

        if visited.contains(&next_node_id) {
            return Err(anyhow::anyhow!("Tournament plan is not a tree"));
        }
        visited.insert(next_node_id);

        let next_node = nodes.iter().find(|n| n.uuid == next_node_id).unwrap();

        match &next_node.config {
            domain::tournament_plan_node::PlanNodeType::Round { config, rounds } => {
                let num_rounds_to_consider = usize::max(rounds.len(), config.num_rounds() as usize);
                let special_name = if num_rounds_to_consider == 1 {
                    let special_name = get_special_name_from_preceding_breaks(&prev_breaks);
                    special_name
                } else {
                    None
                };

                for idx in 0..num_rounds_to_consider {
                    if let Some(special_name) = &special_name {
                        names.insert((next_node_id, idx), special_name.clone());
                    }
                    else {
                        let round_number = curr_idx + idx + 1;
                        names.insert((next_node_id, idx), format!("Round {}", round_number));
                    }
                }
                curr_idx += rounds.len();
                let children = node_children.get(&next_node_id).unwrap_or(&empty_vec);
                for child in children {
                    explore_queue.push((*child, prev_breaks.clone()));
                }
            },
            domain::tournament_plan_node::PlanNodeType::Break { config, break_id: _ } => {
                let children = node_children.get(&next_node_id).unwrap_or(&empty_vec);
                for child in children {
                    let mut breaks = prev_breaks.clone();
                    breaks.push(&config);
                    explore_queue.push((*child, breaks));
                }
            },
        }

    }

    Ok(names)
}

impl TournamentTreeView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let rounds = domain::round::TournamentRound::get_all_in_tournament(db, tournament_uuid).await?;
        let _breaks = domain::tournament_break::TournamentBreak::get_all_in_tournament(db, tournament_uuid).await?;
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

        let names = get_round_names(nodes.clone(), &node_children, &nodes.iter().filter_map(
            |n| {
                if nodes_to_parents.contains_key(&n.uuid) {
                    None
                }
                else {
                    Some(n.uuid)
                }
            }
        ).collect::<Vec<_>>())?;

        let rounds = rounds.into_iter().map(
            |r| {
                (r.uuid, r)
            }
        ).collect::<HashMap<_, _>>();

        let nodes = nodes.into_iter().map(
            |n| {
                (n.uuid, n)
            }
        ).collect::<HashMap<_, _>>();

        let root_nodes = nodes.iter().filter_map(
            |(uuid, node)| {
                if nodes_to_parents.contains_key(uuid) {
                    None
                }
                else {
                    Some(node)
                }
            }
        ).collect::<Vec<_>>();

        let tree_nodes = root_nodes.into_iter().map(
            |node| {
                Box::new(Self::subtree_from_node(node.uuid, &nodes, &rounds,  &node_children, &names))
            }
        ).collect::<Vec<_>>();

        Ok(TournamentTreeView {
            tree: TournamentTreeNode {
                uuid: None,
                content: TournamentTreeNodeContent::Root,
                children: tree_nodes,
                available_actions: vec![
                    AvailableAction {
                        description: "Add Three Preliminary Rounds".to_string(),
                        action: EditTreeActionType::AddPreliminaryRounds { parent: None }
                    },
                ],
            }
        })
    }
    
    fn subtree_from_node(node_uuid: Uuid, nodes: &HashMap<Uuid, TournamentPlanNode>, rounds: &HashMap<Uuid, TournamentRound>, node_children: &HashMap<Uuid, Vec<Uuid>>, names: &HashMap<(Uuid, usize), String>) -> TournamentTreeNode {
        let empty_vec = vec![];
        let children = node_children.get(&node_uuid).unwrap_or(&empty_vec);
        
        let child_nodes = children.iter().map(
            |child_uuid| {
                Self::subtree_from_node(*child_uuid, nodes, rounds, node_children, names)
            }
        ).collect::<Vec<_>>();

        let node = nodes.get(&node_uuid).unwrap();

        let (content, available_actions) = match &node.config {
            domain::tournament_plan_node::PlanNodeType::Round { config, rounds: node_rounds } => {
                let mut actual_rounds = node_rounds.iter().enumerate().filter_map(
                    |(round_idx, round_uuid)| {
                        rounds.get(round_uuid).map(
                            |round| {
                                let name = names.get(&(node_uuid, round_idx)).cloned().unwrap_or("Unknown Round".into());

                                let plan_state = RoundInfoState::Ok;

                                RoundInfo {
                                    uuid: Some(round.uuid),
                                    name,
                                    plan_state
                                }
                            }
                        )
                    }
                ).collect::<Vec<_>>();

                if actual_rounds.len() as i32 >= config.num_rounds() {
                    actual_rounds.iter_mut().enumerate().for_each(|(idx, round)| {
                        if idx >= config.num_rounds() as usize {
                            round.plan_state = RoundInfoState::Superflous;
                        }
                    });
                }
                else {
                    for idx in (actual_rounds.len() as i32)..(config.num_rounds()) {
                        let name = names.get(&(node_uuid, idx as usize)).cloned().unwrap_or("Unknown Round".into());
                        let plan_state = RoundInfoState::Ghost;

                        actual_rounds.push(
                            RoundInfo {
                                uuid: None,
                                name,
                                plan_state
                            }
                        );
                    }
                }
                (TournamentTreeNodeContent::RoundGroup(RoundGroupInfo {
                    rounds: actual_rounds,
                    config: config.clone()
                }),  Self::get_standard_node_actions(node_uuid))
            },
            domain::tournament_plan_node::PlanNodeType::Break { config, break_id } => {
                let break_description = config.human_readable_description();

                (TournamentTreeNodeContent::Break(BreakInfo {
                    uuid: *break_id,
                    break_description,
                    config: config.clone()
                }),  Self::get_standard_node_actions(node_uuid),)
            },
        };

        TournamentTreeNode {
            uuid: Some(node_uuid),
            content,
            children: child_nodes.into_iter().map(|c| Box::new(c)).collect(),
            available_actions
        }
    }

    fn get_standard_node_actions(round_uuid: Uuid) -> Vec<AvailableAction> {
        vec![
            AvailableAction {
                description: "Add Preliminary Rounds".to_string(),
                action: EditTreeActionType::AddPreliminaryRounds { parent: Some(round_uuid) }
            },
            AvailableAction {
                description: "Add Finals".to_string(),
                action: EditTreeActionType::AddKOStage { parent: round_uuid, num_stages: 1 }
            },
            AvailableAction {
                description: "Add Semi-Finals".to_string(),
                action: EditTreeActionType::AddKOStage { parent: round_uuid, num_stages: 2 }
            },
            AvailableAction {
                description: "Add Quarter-Finals".to_string(),
                action: EditTreeActionType::AddKOStage { parent: round_uuid, num_stages: 3 }
            },
            AvailableAction {
                description: "Add Octo-Finals".to_string(),
                action: EditTreeActionType::AddKOStage { parent: round_uuid, num_stages: 4 }
            },
            AvailableAction {
                description: "Add Minor Break (1 round)".to_string(),
                action: EditTreeActionType::AddMinorBreakRounds { parent: round_uuid, draws: vec![
                    FoldDrawConfig {
                        team_fold_method: domain::tournament_plan_node::TeamFoldMethod::BalancedPowerPaired,
                        team_assignment_rule: domain::tournament_plan_node::TeamAssignmentRule::Random,
                        non_aligned_fold_method: domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                    },
                ] }
            },
            AvailableAction {
                description: "Add Minor Break (1 round, balanced)".to_string(),
                action: EditTreeActionType::AddMinorBreakRounds { parent: round_uuid, draws: vec![
                    FoldDrawConfig {
                        team_fold_method: domain::tournament_plan_node::TeamFoldMethod::BalancedPowerPaired,
                        team_assignment_rule: domain::tournament_plan_node::TeamAssignmentRule::Random,
                        non_aligned_fold_method: domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                    },
                ] }
            },
            AvailableAction {
                description: "Add Minor Break (2 rounds)".to_string(),
                action: EditTreeActionType::AddMinorBreakRounds { parent: round_uuid, draws: vec![
                    FoldDrawConfig {
                        team_fold_method: domain::tournament_plan_node::TeamFoldMethod::InversePowerPaired,
                        team_assignment_rule: domain::tournament_plan_node::TeamAssignmentRule::Random,
                        non_aligned_fold_method: domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                    },
                    FoldDrawConfig {
                        team_fold_method: domain::tournament_plan_node::TeamFoldMethod::PowerPaired,
                        team_assignment_rule: domain::tournament_plan_node::TeamAssignmentRule::Random,
                        non_aligned_fold_method: domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                    }
                ] }
            },
            AvailableAction {
                description: "Add Minor Break (2 rounds, balanced)".to_string(),
                action: EditTreeActionType::AddMinorBreakRounds { parent: round_uuid, draws: vec![
                    FoldDrawConfig {
                        team_fold_method: domain::tournament_plan_node::TeamFoldMethod::InversePowerPaired,
                        team_assignment_rule: domain::tournament_plan_node::TeamAssignmentRule::Random,
                        non_aligned_fold_method: domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                    },
                    FoldDrawConfig {
                        team_fold_method: domain::tournament_plan_node::TeamFoldMethod::BalancedPowerPaired,
                        team_assignment_rule: domain::tournament_plan_node::TeamAssignmentRule::Random,
                        non_aligned_fold_method: domain::tournament_plan_node::NonAlignedFoldMethod::Random,
                    }
                ] }
            },
            AvailableAction {
                description: "Add Reitze Break".to_string(),
                action: EditTreeActionType::AddTimBreakRounds { parent: round_uuid }
            },
        ]
    }
}