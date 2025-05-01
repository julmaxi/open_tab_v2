use open_tab_entities::domain::{
    tournament_plan_edge::TournamentPlanEdge,
    tournament_plan_node::{FoldDrawConfig, PlanNodeType, TournamentPlanNode},
};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentCreationConfig {
    pub name: String,
    pub num_preliminaries: u32,
    pub num_break_rounds: u32,
    pub use_default_feedback_system: bool,
}

impl TournamentCreationConfig {
    pub fn get_tournament_graph(
        &self,
        tournament_id: Uuid,
    ) -> (Vec<TournamentPlanNode>, Vec<TournamentPlanEdge>) {
        let mut all_edges = Vec::new();
        let mut all_nodes = Vec::new();
        let num_prelim_roundtrips = (self.num_preliminaries / 3) as i32;

        let prelim_node = TournamentPlanNode::new(
            tournament_id,
            PlanNodeType::Round { config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::Preliminaries { num_roundtrips: num_prelim_roundtrips }, rounds: vec![] }
        );

        let prelim_node_uuid = prelim_node.uuid;
        all_nodes.push(prelim_node);

        let final_node_id = if self.num_preliminaries % 3 != 0 {
            let minor_break_node = TournamentPlanNode::new(
                tournament_id,
                PlanNodeType::Break {
                    config:
                        open_tab_entities::domain::tournament_plan_node::BreakConfig::TwoThirdsBreak,
                    break_id: None,
                    suggested_award_title: None,
                    eligible_categories: vec![],
                    max_breaking_adjudicator_count: None,
                    is_only_award: false,
                    suggested_break_award_prestige: None,
                    suggested_award_series_key: None,
                },
            );
            let minor_break_node_uuid = minor_break_node.uuid;

            let (minor_break_subtree_start_id, minor_break_subtree_end_id, nodes, edges) = if self
                .num_preliminaries
                % 3
                == 1
            {
                let minor_break_round = TournamentPlanNode::new(
                    tournament_id,
                    PlanNodeType::Round { config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        round_configs: vec![
                            FoldDrawConfig {
                                team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::Random,
                                team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random,
                                non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random
                            }
                        ]
                    }, rounds: vec![] }
                );
                (
                    minor_break_round.uuid,
                    minor_break_round.uuid,
                    vec![minor_break_round],
                    vec![],
                )
            } else {
                let first_round = TournamentPlanNode::new(
                    tournament_id,
                    PlanNodeType::Round { config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        round_configs: vec![
                            FoldDrawConfig {
                                team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::HalfRandom,
                                team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random,
                                non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random
                            }
                        ]
                    }, rounds: vec![] }
                );

                let break_ = TournamentPlanNode::new(
                    tournament_id,
                    PlanNodeType::Break {
                        config:
                            open_tab_entities::domain::tournament_plan_node::BreakConfig::TimBreak,
                        break_id: None,
                        eligible_categories: vec![],
                        suggested_award_title: None,
                        max_breaking_adjudicator_count: None,
                        is_only_award: false,
                        suggested_break_award_prestige: None,
                        suggested_award_series_key: None,
                    },
                );

                let second_round = TournamentPlanNode::new(
                    tournament_id,
                    PlanNodeType::Round { config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw {
                        round_configs: vec![
                            FoldDrawConfig {
                                team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::BalancedPowerPaired,
                                team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::InvertPrevious,
                                non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random
                            }
                        ]
                    }, rounds: vec![] }
                );

                let first_uuid = first_round.uuid;
                let second_uuid = second_round.uuid;
                let break_uuid = break_.uuid;

                (
                    first_uuid,
                    second_uuid,
                    vec![first_round, second_round, break_],
                    vec![(first_uuid, break_uuid), (break_uuid, second_uuid)],
                )
            };

            all_nodes.push(minor_break_node);
            all_nodes.extend(nodes);
            all_edges.push(TournamentPlanEdge::new(
                prelim_node_uuid,
                minor_break_node_uuid,
            ));
            all_edges.push(TournamentPlanEdge::new(
                minor_break_node_uuid,
                minor_break_subtree_start_id,
            ));

            for (src, tgt) in edges {
                all_edges.push(TournamentPlanEdge::new(src, tgt));
            }

            minor_break_subtree_end_id
        } else {
            prelim_node_uuid
        };

        let mut prev_id: Uuid = final_node_id;

        for break_round_idx in 0..self.num_break_rounds {
            let num_debates = u32::pow(2, self.num_break_rounds - break_round_idx - 1);

            let break_node = TournamentPlanNode::new(
                tournament_id,
                PlanNodeType::Break {
                    config: if break_round_idx == 0 {
                        open_tab_entities::domain::tournament_plan_node::BreakConfig::TabBreak {
                            num_teams: num_debates * 2,
                            num_non_aligned: num_debates * 3,
                        }
                    } else {
                        open_tab_entities::domain::tournament_plan_node::BreakConfig::KnockoutBreak
                    },
                    break_id: None,
                    suggested_award_title: if break_round_idx == 0 {
                        Some("Open".to_string())
                    } else {
                        None
                    },
                    eligible_categories: vec![],
                    max_breaking_adjudicator_count: None,
                    is_only_award: false,
                    suggested_break_award_prestige: Some(0),
                    suggested_award_series_key: None,
                },
            );
            let break_node_id = break_node.uuid;

            let node = TournamentPlanNode::new(
                tournament_id,
                PlanNodeType::Round { config: open_tab_entities::domain::tournament_plan_node::RoundGroupConfig::FoldDraw { round_configs: vec![
                    FoldDrawConfig {
                        team_fold_method: open_tab_entities::domain::tournament_plan_node::TeamFoldMethod::InversePowerPaired,
                        team_assignment_rule: open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random,
                        non_aligned_fold_method: open_tab_entities::domain::tournament_plan_node::NonAlignedFoldMethod::Random
                    }
                ]}, rounds: vec![] 
                }
            );

            all_edges.push(TournamentPlanEdge::new(prev_id, break_node_id));
            all_edges.push(TournamentPlanEdge::new(break_node_id, node.uuid));
            prev_id = node.uuid;
            all_nodes.push(break_node);
            all_nodes.push(node);
        }

        let award_node = TournamentPlanNode::new(
            tournament_id,
            PlanNodeType::Break {
                config: open_tab_entities::domain::tournament_plan_node::BreakConfig::TeamOnlyKnockoutBreak,
                break_id: None,
                eligible_categories: vec![],
                suggested_award_title: Some("Best Team".to_string()),
                suggested_break_award_prestige: Some(100),
                max_breaking_adjudicator_count: None,
                is_only_award: true,
                suggested_award_series_key: None
            }
        );

        all_edges.push(TournamentPlanEdge::new(prev_id, award_node.uuid));
        all_nodes.push(award_node);

        (all_nodes, all_edges)
    }
}
