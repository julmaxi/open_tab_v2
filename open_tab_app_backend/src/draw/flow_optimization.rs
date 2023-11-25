use std::{collections::{HashMap, HashSet}, sync::Arc};

use itertools::Itertools;
use open_tab_entities::{domain::{self}, prelude::{Ballot, Participant, TournamentRound}};
use sea_orm::{prelude::Uuid};

use mcmf::{GraphBuilder, Capacity, Vertex, Cost};

use crate::draw_view::DrawBallot;

use super::evaluation::DrawEvaluator;

use super::datastructures::{
    AdjudicatorInfo, RoundInfo, DebateInfo
};

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum AdjudicatorPosition {
    None,
    Chair{debate_idx: usize},
    Wing{debate_idx: usize, position: usize},
    Unavailable
}

#[derive(Debug, Clone)]
pub struct OptimizationOptions {
    feedback_weight: f32,
    moderation_weight: f32,
    max_discussion_improvement_weight: f32,
    moderation_score_difference_weight: f32,

    hard_clash_threshold: i32,
    bias_weight: f32,
    variance_weight: f32,
}

impl Default for OptimizationOptions {
    fn default() -> Self {
        Self {
            feedback_weight: 1.0,
            moderation_weight: 1.0,
            max_discussion_improvement_weight: 1.0,
            moderation_score_difference_weight: 1.0,

            bias_weight: 1.0,
            variance_weight: 1.0,

            hard_clash_threshold: 75,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizationState {
    pub rounds: Vec<RoundInfo>,

    options: Arc<OptimizationOptions>,

    adjudicator_assignments: HashMap<Uuid, Vec<AdjudicatorPosition>>,
    adjudicator_info: HashMap<Uuid, AdjudicatorInfo>,

    evaluator: Arc<DrawEvaluator>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum NodeType {
    Adjudicator(Uuid),
    AdjudicatorChairFrequency(Uuid, i32),
    AdjudicatorRoundRole(Uuid, usize),
    Debate(usize, usize)
}

impl OptimizationState {
    pub async fn load_from_round_ids<C>(db: &C, tournament_id: Uuid, round_ids: Vec<Uuid>, options: Arc<OptimizationOptions>, evaluator: Arc<DrawEvaluator>) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let rounds = RoundInfo::load_from_rounds(db, round_ids).await?;

        Self::load_from_rounds(db, tournament_id, rounds, options, evaluator).await
    }

    pub async fn load_from_rounds_and_ballots<C>(db: &C, tournament_id: Uuid, round_draw: Vec<(TournamentRound, Vec<Ballot>)>, options: Arc<OptimizationOptions>, evaluator: Arc<DrawEvaluator>) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let rounds = round_draw.into_iter().map(
            |(round_, draws)| {
                RoundInfo {
                    id: round_.uuid,
                    is_silent: round_.is_silent,
                    debates: draws.into_iter().map(
                        Ballot::into
                    ).collect_vec()
                }
            } 
        ).collect_vec();

        Self::load_from_rounds(db, tournament_id, rounds, options, evaluator).await
    }

    pub async fn load_from_rounds_and_draw_ballots<C>(db: &C, tournament_id: Uuid, round_draw: Vec<(&TournamentRound, &Vec<DrawBallot>)>, options: Arc<OptimizationOptions>, evaluator: Arc<DrawEvaluator>) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let rounds = round_draw.into_iter().map(
            |(round_, draws)| {
                RoundInfo {
                    id: round_.uuid,
                    is_silent: round_.is_silent,
                    debates: draws.into_iter().map(
                        DebateInfo::from
                    ).collect_vec()
                }
            } 
        ).collect_vec();

        Self::load_from_rounds(db, tournament_id, rounds, options, evaluator).await
    }

    pub async fn load_from_rounds<C>(db: &C, tournament_id: Uuid, rounds: Vec<RoundInfo>, options: Arc<OptimizationOptions>, evaluator: Arc<DrawEvaluator>) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let adjudicators = Participant::get_all_adjudicators_in_tournament(db, tournament_id).await?;

        let mut adjudicator_assignments = adjudicators.iter().map(|adj| (adj.uuid.clone(), vec![AdjudicatorPosition::None; rounds.len()])).collect::<HashMap<Uuid, Vec<AdjudicatorPosition>>>();
        rounds.iter().enumerate().for_each(|(round_idx, round)| {
            round.debates.iter().enumerate().flat_map(|(debate_idx, debate_info)| {
                debate_info.chair.iter().map(
                    move |chair_uuid| (chair_uuid, AdjudicatorPosition::Chair { debate_idx })
                ).chain(
                    debate_info.wings.iter().enumerate().map(
                        move |(adj_pos, adj_id)| (adj_id, AdjudicatorPosition::Wing { debate_idx, position: adj_pos })
                    )
                )
            }).for_each(
                |(adj_id, position)| {
                    adjudicator_assignments.get_mut(adj_id).unwrap()[round_idx] = position;
                }
            );
        });

        let round_id_to_pos = rounds.iter().enumerate().map(
            |(round_idx, round_)| {
                (round_.id, round_idx)
            }
        ).collect::<HashMap<Uuid, usize>>();

        for adjudicator in adjudicators.iter() {
            match &adjudicator.role {
                domain::participant::ParticipantRole::Adjudicator(info) => {
                    for r in info.unavailable_rounds.iter() {
                        if let Some(r_idx) = round_id_to_pos.get(&r) {
                            adjudicator_assignments.get_mut(&adjudicator.uuid).unwrap()[*r_idx] = AdjudicatorPosition::Unavailable;
                        }
                    }
                }
                _ => {}
            }
        }

        let adjudicator_info = adjudicators.iter().filter_map(|adj| {
            match &adj.role {
                domain::participant::ParticipantRole::Adjudicator(info) => Some(
                    (adj.uuid, AdjudicatorInfo {
                        id: adj.uuid,
                        feedback_skill: info.chair_skill as i32,
                        moderation_skill: info.chair_skill as i32,
                        discussion_skill: info.panel_skill as i32,
                        bias: 0.0,
                        variance: 0.0,
                    }
                    )
                ),
                _ => None
            }
        }).collect();

        Ok(
            Self { rounds, options, adjudicator_assignments, adjudicator_info, evaluator }
        )
    }

    fn compute_adjudicator_chair_cost_in_debate(&self, adjudicator: Uuid, debate: &DebateInfo, is_silent_round: bool) -> Option<i32> {
        let adj_info = self.adjudicator_info.get(&adjudicator).unwrap();
        let cost = self.compute_clash_cost_in_debate(adj_info, debate);
        if let Some(mut cost) = cost {
            if !is_silent_round {
                cost -= (adj_info.feedback_skill as f32 * self.options.feedback_weight).round() as i32;
            }
            cost -= (adj_info.moderation_skill as f32 * self.options.moderation_weight).round() as i32;
    
            Some(cost)
        }
        else {
            None
        }
    }

    fn compute_clash_cost_in_debate(&self, adj_info: &AdjudicatorInfo, debate: &DebateInfo) -> Option<i32> {
        let mut debate = debate.clone();
        debate.wings.push(adj_info.id);
        let scores = self.evaluator.find_issues_in_debate(&debate);
        let empty = vec![];
        let adj_issues = scores.adjudicator_issues.get(&adj_info.id).unwrap_or(&empty);
        if adj_issues.iter().any(|i| i.severity as i32 >= self.options.hard_clash_threshold) {
            return None;
        }
        Some(adj_issues.into_iter().map(|issue| issue.severity as i32 * 100).sum())
    }

    fn compute_wing_cost_in_debate(&self, adjudicator: Uuid, debate: &DebateInfo) -> Option<i32> {
        let adj_info = self.adjudicator_info.get(&adjudicator).unwrap();
        let cost = self.compute_clash_cost_in_debate(adj_info, debate);

        if let Some(mut cost) = cost {
            let avg_wing_discussion_skill = if debate.wings.len() > 0 {
                debate.wings.iter().map(|w| self.adjudicator_info.get(w).unwrap().discussion_skill).sum::<i32>() / debate.wings.len() as i32
            } else {
                0
            };

            let chair_info = debate.chair.map(|c| self.adjudicator_info.get(&c).unwrap());

            let mut bias_sum = debate.wings.iter().map(|w| self.adjudicator_info.get(w).unwrap().bias).sum::<f32>();
            let mut variance_sum = debate.wings.iter().map(|w| self.adjudicator_info.get(w).unwrap().variance).sum::<f32>();

            if let Some(chair_info) = chair_info {
                cost -= ((adj_info.moderation_skill - chair_info.discussion_skill) as f32 * self.options.max_discussion_improvement_weight).round() as i32;

                bias_sum += chair_info.bias;
                variance_sum += chair_info.variance;
            }

            let avg_bias = bias_sum / (debate.wings.len() + chair_info.is_some() as usize) as f32;
            let avg_variance = variance_sum / (debate.wings.len() + chair_info.is_some() as usize) as f32;

            cost -= ((adj_info.discussion_skill - avg_wing_discussion_skill).abs() as f32 * self.options.moderation_score_difference_weight) as i32;
            cost -= ((adj_info.bias - avg_bias).abs() as f32 * self.options.bias_weight) as i32;
            cost -= ((avg_variance - adj_info.variance) as f32 * self.options.variance_weight) as i32;

            Some(cost)      
        }
        else {
            None
        }
    }

    pub fn update_state_by_assigning_chairs(&mut self) {
        let debates_to_assign_chair = (0..self.rounds.len()).into_iter().flat_map(
            |i| self.rounds[i].debates.iter().enumerate().filter(|(_d_idx, d)| d.chair.is_none()).map(move |(d_idx, d)| (i, d_idx, d))
        ).collect_vec();

        let mut graph_build = GraphBuilder::new();

        self.adjudicator_assignments.keys().for_each(
            |adj| {
                for i in 0..self.rounds.len() {
                    graph_build.add_edge(
                        Vertex::Source,
                        NodeType::AdjudicatorChairFrequency(*adj, i as i32),
                        Capacity(1),
                        Cost(i as i32 * 10)
                    );
                    graph_build.add_edge(
                        NodeType::AdjudicatorChairFrequency(*adj, i as i32),
                        NodeType::Adjudicator(*adj),
                        Capacity(1), // This overestimates the capacity
                        Cost(0)
                    );    
                }
            }
        );

        (0..self.rounds.len()).into_iter().for_each(
            |round_id| {
                self.adjudicator_assignments.iter().for_each(
                    |(adj_id, assignments)| {
                        if assignments[round_id] == AdjudicatorPosition::None {
                            graph_build.add_edge(
                                NodeType::Adjudicator(*adj_id),
                                NodeType::AdjudicatorRoundRole(*adj_id, round_id),
                                Capacity(1),
                                Cost(0)
                            );
                        }
                    }
                )
            }
        );

        debates_to_assign_chair.iter().for_each(
            |(round_id, debate_idx, debate)| {
                let is_silent = self.rounds[*round_id].is_silent;
                self.adjudicator_assignments.keys().for_each(
                    |adj_id| {
                        if let Some(cost) = self.compute_adjudicator_chair_cost_in_debate(*adj_id, debate, is_silent) {
                            graph_build.add_edge(
                                NodeType::AdjudicatorRoundRole(*adj_id, *round_id),
                                NodeType::Debate(*round_id, *debate_idx),
                                Capacity(1),
                                Cost(
                                    cost
                                )
                            );    
                        }
                    }
                )
            }
        );

        //Add sink
        debates_to_assign_chair.iter().for_each(
            |(round_id, debate_idx, _debate)| {
                graph_build.add_edge(
                    NodeType::Debate(*round_id, *debate_idx),
                    Vertex::Sink,
                    Capacity(1),
                    Cost(0)
                );
            }
        );

        let (_cost, paths) = graph_build.mcmf();
        let all_assignments = paths.iter().flat_map(
            |path| path.edges()
        ).filter_map(
            |edge| {
                if edge.amount > 0 {
                    match (&edge.a, &edge.b) {
                        (Vertex::Node(NodeType::AdjudicatorRoundRole(adj, round_id)), Vertex::Node(NodeType::Debate(_, debate_id))) => Some((adj, round_id, debate_id)),
                        _ => None
                    }    
                }
                else {
                    None
                }
            }
        );

        all_assignments.for_each(
            |(adj, round_id, debate_id)| {
                self.adjudicator_assignments.get_mut(adj).unwrap()[*round_id] = AdjudicatorPosition::Chair{debate_idx: *debate_id};
                self.rounds[*round_id].debates[*debate_id].chair = Some(*adj);
            }
        );
    }

    pub fn update_state_by_assigning_wings(&mut self) {
        for round_id in 0..self.rounds.len() {
            let mut previous_unassigned_cnt = self.adjudicator_assignments.len() + 1;
            loop {
                let round_info = &self.rounds[round_id];
                let mut unassigned_adjudicators : HashSet<Uuid> = self.adjudicator_assignments.iter().filter_map(
                    |(adj, assignments)| {
                        if assignments[round_id] == AdjudicatorPosition::None {
                            Some(*adj)
                        }
                        else {
                            None
                        }
                    }
                ).collect();

                if unassigned_adjudicators.len() == 0 || unassigned_adjudicators.len() == previous_unassigned_cnt {
                    break;
                }

                previous_unassigned_cnt = unassigned_adjudicators.len();

                let min_debate_wing_cnt: usize = round_info.debates.iter().map(|d| d.wings.len()).min().unwrap_or(0);
                let debates_to_assign_wings = round_info.debates.iter().enumerate().filter(|(_d_idx, d)| d.wings.len() == min_debate_wing_cnt).collect_vec();

                let mut graph_build = GraphBuilder::new();

                unassigned_adjudicators.iter().for_each(
                    |adj| {
                        graph_build.add_edge(
                            Vertex::Source,
                            NodeType::Adjudicator(*adj),
                            Capacity(1),
                            Cost(0)
                        );

                        debates_to_assign_wings.iter().for_each(
                            |(debate_idx, debate)| {
                                if let Some(cost) = self.compute_wing_cost_in_debate(*adj, debate) {
                                    graph_build.add_edge(
                                        NodeType::Adjudicator(*adj),
                                        NodeType::Debate(round_id, *debate_idx),
                                        Capacity(1),
                                        Cost(
                                            cost
                                        )
                                    );
                                }
                            }
                        );
                    }
                );
                debates_to_assign_wings.iter().for_each(
                    |(debate_idx, _debate)| {
                        graph_build.add_edge(
                            NodeType::Debate(round_id, *debate_idx),
                            Vertex::Sink,
                            Capacity(1),
                            Cost(0)
                        );
                    }
                );

                let (_cost, paths) = graph_build.mcmf();

                let assignments = paths.iter().flat_map(
                    |path| path.edges()
                ).filter_map(
                    |edge| {
                        if edge.amount > 0 {
                            match (&edge.a, &edge.b) {
                                (Vertex::Node(NodeType::Adjudicator(adj)), Vertex::Node(NodeType::Debate(_, debate_id))) => Some((adj, debate_id)),
                                _ => None
                            }    
                        }
                        else {
                            None
                        }
                    }
                ).collect_vec();

                assignments.into_iter().for_each(
                    |(adj, debate_id)| {
                        self.adjudicator_assignments.get_mut(adj).unwrap()[round_id] = AdjudicatorPosition::Wing{debate_idx: *debate_id, position: self.rounds[round_id].debates[*debate_id].wings.len()};
                        self.rounds[round_id].debates[*debate_id].wings.push(*adj);
                        unassigned_adjudicators.remove(adj);
                    }
                );
            }
        }
    }

    pub fn update_state_by_assigning_adjudicators(&mut self) {
        self.update_state_by_assigning_chairs();
        self.update_state_by_assigning_wings();
    }
}
