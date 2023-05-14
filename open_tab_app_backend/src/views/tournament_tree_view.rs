use open_tab_entities::domain::round::DrawType;
use open_tab_entities::domain::tournament_break::{TournamentBreak};
use sea_orm::prelude::Uuid;


use std::vec;
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;

use open_tab_entities::domain;


use itertools::Itertools;

use crate::draw::preliminary::MinorBreakRoundDrawType;
use crate::{LoadedView, EditTreeActionType};

pub struct LoadedTournamentTreeView {
    pub view: TournamentTreeView,
    pub tournament_id: Uuid
}

impl LoadedTournamentTreeView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
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
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.tournament_rounds.len() > 0 {
            self.view = TournamentTreeView::load_from_tournament(db, self.tournament_id).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TournamentTreeNode {
    content: TournamentTreeNodeContent,
    children: Vec<Box<TournamentTreeNode>>,
    available_actions: Vec<AvailableAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AvailableAction {
    description: String,
    action: EditTreeActionType,
}

/*
impl From<EditTreeActionType> for AvailableAction {
    fn from(action: EditTreeActionType) -> Self {
        match action {
            EditTreeActionType::AddThreePreliminaryRounds { .. } => {
                AvailableAction {
                    description: "Add Three Preliminary Rounds".to_string(),
                    action
                }
            }
        }
    }
}
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoundInfo {
    uuid: Uuid,
    round_number: i32,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BreakInfo {
    uuid: Uuid,
    break_description: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoundGroupInfo {
    rounds: Vec<RoundInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum TournamentTreeNodeContent {
    Root,
    Round(RoundInfo),
    RoundGroup(RoundGroupInfo),
    Break(BreakInfo),
    Error
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentTreeView {
    tree: TournamentTreeNode,
}

enum BreakOrRound {
    Break(TournamentBreak),
    Round(TournamentRound),
}

impl TournamentTreeView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        let rounds = domain::round::TournamentRound::get_all_in_tournament(db, tournament_uuid).await?;
        let breaks = domain::tournament_break::TournamentBreak::get_all_in_tournament(db, tournament_uuid).await?;

        let rounds = rounds.into_iter().map(
            |r| {
                (r.uuid, r)
            }
        ).collect::<HashMap<_, _>>();

        let round_breaks = breaks.iter().flat_map(
            |b| {
                b.child_rounds.iter().map(
                    |r| (*r, b.uuid)
                )
            }
        ).collect::<HashMap<_, _>>();

        let last_rounds_before_break_map : Result<HashMap<_, _>, &'static str> = breaks.iter().map(
            |b: &domain::tournament_break::TournamentBreak| {
                let source_rounds : Result<Vec<_>, _> = b.source_rounds.iter().map(|r| rounds.get(&r.uuid).ok_or("Round missing")).collect();
                let mut source_rounds = source_rounds?;
                source_rounds.sort_by_key(|r| -(r.index as i32));
                let last_round = source_rounds.into_iter().next().ok_or("No source");
                Ok((b.uuid, last_round?.uuid))
            }
        ).collect();

        let mut rounds_by_break_requirements = rounds.values().sorted_by_key(|r| r.index).map(
            |r| {
                (round_breaks.get(&r.uuid).map(|u| *u), r)
            }
        ).into_group_map();

        let tree_children = if rounds_by_break_requirements.len() > 0 {

            for entry in rounds_by_break_requirements.iter_mut() {
                entry.1.sort_by_key(|r| r.index);
            }
    
            let mut children = HashMap::new();
    
            //Every break is a child of the last round before it
            for (break_uuid, round_uuid) in last_rounds_before_break_map? {
                children.entry(round_uuid).or_insert_with(Vec::new).push(
                    break_uuid
                );
            }
    
            //Every round is a child of the round immediately preceding it according to index
            //provided it is in the same break
            for (_, rounds) in rounds_by_break_requirements.iter() {
                let mut rounds = rounds.into_iter().peekable();
                while let Some(round) = rounds.next() {
                    if let Some(next_round) = rounds.peek() {
                        children.entry(round.uuid).or_insert_with(Vec::new).push(
                            next_round.uuid
                        );
                    }
                }
            }
    
            let first_round_uuid = rounds_by_break_requirements.get(&None).and_then(|r| r.first()).map(|r| r.uuid);
    
            if !first_round_uuid.is_some() {
                return Err("No first round".into());
            }
            let first_round_uuid = first_round_uuid.unwrap();
    
            let breaks_and_round_by_uuid = breaks.into_iter().map(
                |b| {
                    (b.uuid, BreakOrRound::Break(b))
                }
            ).chain(
                rounds.clone().into_iter().map(
                    |r| {
                        (r.0, BreakOrRound::Round(r.1))
                    }
                )
            ).collect::<HashMap<_, _>>();
    
            //The first round in a break is a child of the break
            for (break_uuid, rounds) in rounds_by_break_requirements {
                if break_uuid.is_none() {
                    continue;
                }
                let mut rounds = rounds.into_iter().sorted_by_key(|r| r.index).peekable();
                if let Some(round) = rounds.next() {
                    children.entry(break_uuid.unwrap()).or_insert_with(Vec::new).push(
                        round.uuid
                    );
                }
            }
            vec![Box::new(Self::subtree_from_node(first_round_uuid, &children, &breaks_and_round_by_uuid))]
        }
        else {
            vec![]
        };
        
        Ok(TournamentTreeView {
            tree: TournamentTreeNode {
                content: TournamentTreeNodeContent::Root,
                children: tree_children,
                available_actions: vec![
                    AvailableAction {
                        description: "Add Three Preliminary Rounds".to_string(),
                        action: EditTreeActionType::AddThreePreliminaryRounds { parent: None }
                    },
                ],
            }
        })
    }
    
    fn subtree_from_node(node_uuid: Uuid, all_children: &HashMap<Uuid, Vec<Uuid>>, breaks_and_round_by_uuid: &HashMap<Uuid, BreakOrRound>) -> TournamentTreeNode {
        let children = all_children.get(&node_uuid);
        let content = breaks_and_round_by_uuid.get(&node_uuid);

        let mut local_children = children.map(|c| c.clone()).unwrap_or_else(Vec::new);

        let node_content = if let Some(BreakOrRound::Round(parent_round)) = content {
            let mut collated_children = Vec::new();
            if parent_round.draw_type == Some(DrawType::StandardPreliminaryDraw) {
                let mut unexplored_children: Vec<Uuid> = local_children.clone();
                local_children.clear(); // We need to reconstruct the children for grouping
                while unexplored_children.len() > 0 && collated_children.len() < 2 {
                    let child_uuid = unexplored_children.pop().unwrap();
                    let child = breaks_and_round_by_uuid.get(&child_uuid);

                    let mut did_collate_child = false;
                    if let Some(BreakOrRound::Round(child)) = child {
                        if child.draw_type == Some(DrawType::StandardPreliminaryDraw) {
                            did_collate_child = true;
                            if let Some(c) = all_children.get(&child_uuid) {
                                unexplored_children.extend(c);
                            }
                            collated_children.push(child_uuid);
                        }
                    }

                    if !did_collate_child {
                        local_children.push(child_uuid);
                    }
                }

                local_children.extend(unexplored_children);
            }

            let collated_children = collated_children.into_iter().filter_map(
                |child_uuid| {
                let child = breaks_and_round_by_uuid.get(&child_uuid);
                if let Some(BreakOrRound::Round(child)) = child {
                    Some(
                        RoundInfo {
                            uuid: child_uuid,
                            name: format!("Runde {}", child.index + 1),
                            round_number: child.index as i32,
                            //draw_type: child.draw_type,
                            //children: TournamentTreeNode::subtree_from_node(child_uuid, all_children, breaksAndRoundByUuid)
                        }
                    )
                }
                else {
                    None
                }
            }).collect_vec();

            let base_info = RoundInfo {
                uuid: node_uuid,
                name: format!("Runde {}", parent_round.index + 1),
                round_number: parent_round.index as i32,
            };
            if collated_children.len() > 0 {
                let grouped_rounds = vec![base_info].into_iter().chain(collated_children).collect_vec();
                TournamentTreeNodeContent::RoundGroup(
                    RoundGroupInfo { rounds: grouped_rounds }
                )
            }
            else {
                TournamentTreeNodeContent::Round(
                    base_info
                )
            }
        }
        else if let Some(BreakOrRound::Break(parent_break)) = content {
            TournamentTreeNodeContent::Break(
                BreakInfo {
                    uuid: parent_break.uuid,
                    break_description: parent_break.break_type.human_readable_description(),
                }
            )
        }
        else  {
            TournamentTreeNodeContent::Error
        };

        let child_trees = local_children.into_iter().map(
            |child_uuid| {
                Box::new(Self::subtree_from_node(child_uuid, all_children, breaks_and_round_by_uuid))
            }
        ).collect_vec();

        TournamentTreeNode {
            children: child_trees,
            available_actions: match &node_content {
                TournamentTreeNodeContent::RoundGroup(g) => Self::get_standard_round_actions(g.rounds.last().unwrap().uuid),
                TournamentTreeNodeContent::Round(r) => Self::get_standard_round_actions(r.uuid),
                TournamentTreeNodeContent::Break(_b) => vec![],
                _ => vec![] // These never appear in this recursion
            },
            content: node_content,
        }

            /*
            // We group preliminaries without any other children into a group
            let group_children = if let Some(BreakOrRound::Round(round)) = content {
                if let Some(BreakOrRound::Round(child_round)) = children {
                    if child_round.draw_type == Some(DrawType::StandardPreliminaryDraw) && children.len() == 1 {
                        let child = breaksAndRoundByUuid.get(&children[0]);
                        match child {
                            Some(BreakOrRound::Round(round)) => {
                                round.draw_type == Some(DrawType::StandardPreliminaryDraw)
                            }
                            _ => false
                        }
                    }
                    else {
                        false
                    };
                }
                else {
                    false
                }
            }
             */
    }

    fn get_standard_round_actions(round_uuid: Uuid) -> Vec<AvailableAction> {
        vec![
            AvailableAction {
                description: "Add Three Preliminary Rounds".to_string(),
                action: EditTreeActionType::AddThreePreliminaryRounds { parent: Some(round_uuid) }
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
                    MinorBreakRoundDrawType::PowerPaired
                ] }
            },
            AvailableAction {
                description: "Add Minor Break (1 round, balanced)".to_string(),
                action: EditTreeActionType::AddMinorBreakRounds { parent: round_uuid, draws: vec![
                    MinorBreakRoundDrawType::BalancedPowerPaired
                ] }
            },
            AvailableAction {
                description: "Add Minor Break (2 rounds)".to_string(),
                action: EditTreeActionType::AddMinorBreakRounds { parent: round_uuid, draws: vec![
                    MinorBreakRoundDrawType::InversePowerPaired,
                    MinorBreakRoundDrawType::PowerPaired
                ] }
            },
            AvailableAction {
                description: "Add Minor Break (2 rounds, balanced)".to_string(),
                action: EditTreeActionType::AddMinorBreakRounds { parent: round_uuid, draws: vec![
                    MinorBreakRoundDrawType::InversePowerPaired,
                    MinorBreakRoundDrawType::BalancedPowerPaired
                ] }
            },
            AvailableAction {
                description: "Add Reitze Break".to_string(),
                action: EditTreeActionType::AddTimBreakRounds { parent: round_uuid }
            },
        ]
    }
}