use std::collections::{HashSet, VecDeque, HashMap};

use itertools::Itertools;
use open_tab_entities::{domain::{self, debate::TournamentDebate, participant::Participant, round::TournamentRound, tournament_plan_edge::TournamentPlanEdge, tournament_plan_node::TournamentPlanNode}, schema, EntityGroup, EntityTypeId};
use sea_orm::{prelude::*, sea_query::{Query, SqliteQueryBuilder}, IntoSimpleExpr, QuerySelect};
use serde::Serialize;

use crate::LoadedView;
use async_trait::async_trait;

pub struct LoadedProgressView {
    tournament_id: Uuid,
    view: ProgressView,
}

impl LoadedProgressView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            Self {
                tournament_id: tournament_uuid,
                view: ProgressView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedProgressView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        let mut partial_changes = HashMap::new();
        if changes.has_changes_for_types(vec![
            EntityTypeId::Participant,
            EntityTypeId::TournamentDebate,
            EntityTypeId::TournamentRound,
            EntityTypeId::TournamentPlanNode,
            EntityTypeId::TournamentPlanEdge,
            EntityTypeId::Ballot,
        ]) {
            let view = ProgressView::load_from_tournament(db, self.tournament_id).await?;
            self.view = view;
            partial_changes.insert(".".to_string(), serde_json::to_value(&self.view)?);
        }

        if partial_changes.len() > 0 {
            Ok(Some(partial_changes))
        } else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Serialize)]
struct ProgressView {
    steps: Vec<Step>,
}

impl ProgressView {
    pub async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let query = Expr::exists(Query::select().column(schema::participant::Column::TournamentId).from(schema::participant::Entity.table_ref()).and_where(
            schema::participant::Column::TournamentId.eq(tournament_uuid)
        ).take()).to_owned();

        let has_participants = schema::participant::Entity::find().select_only().expr(query).filter(
            schema::participant::Column::TournamentId.eq(tournament_uuid)
        ).distinct().into_tuple::<bool>().one(db).await?.unwrap_or(false);

        let mut steps = vec![];
        steps.push(Step::LoadParticipants {is_done: has_participants});

        if has_participants {
            let all_rounds_by_id = TournamentRound::get_all_in_tournament(db, tournament_uuid).await?.into_iter().map(|r| (r.uuid, r)).collect::<HashMap<_, _>>();
            let all_nodes = TournamentPlanNode::get_all_in_tournament(db, tournament_uuid).await?;
            let all_edges = TournamentPlanEdge::get_all_for_sources(db, all_nodes.iter().map(|n| n.uuid).collect_vec()).await?;

            /*
            let all_debates = TournamentDebate::get_all_in_rounds(db, all_rounds_by_id.keys().cloned().collect_vec()).await?;
            let all_debate_ids = all_debates.iter().flatten().map(|d| d.uuid).collect::<HashSet<_>>();
            let all_round_debate_ids : HashMap<_, _> = all_debates.iter().flatten().map(|d| (d.round_id, d.uuid)).into_group_map();
            let all_ballots_by_debate_id : HashMap<_, _> = domain::ballot::Ballot::get_all_in_debates(db, all_debate_ids.into_iter().collect()).await?.into_iter().collect();
            */
    
            let child_nodes : HashSet<Uuid> = all_edges.iter().map(|e| e.target_id).collect();
            let roots = all_nodes.iter().filter(|n| !child_nodes.contains(&n.uuid)).map(|n| n.uuid).collect_vec();
            let children = all_edges.iter().map(|e| (e.source_id, e.target_id)).into_group_map();

            let all_nodes_by_id = all_nodes.iter().map(|n| (n.uuid, n)).collect::<HashMap<_, _>>();
    
            let mut explore_queue : VecDeque<(Uuid, Option<Uuid>)> = VecDeque::from_iter(roots.into_iter().map(|r| (r, None)));
            let mut has_seen_round_node = false;

            let mut all_nodes_complete = true;

            let all_debates = TournamentDebate::get_all_in_rounds(db, all_rounds_by_id.keys().cloned().collect_vec()).await?;
            let all_debates_by_round_id = all_debates.into_iter().flatten().map(|d| (d.round_id, d)).into_group_map();
            
            while let Some((next, prev_break)) = explore_queue.pop_front() {
                let node = all_nodes_by_id.get(&next).expect("Db constraint failed");
                let empty = vec![];
                let children = children.get(&next).unwrap_or(&empty);

                match &node.config {
                    open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round { config, rounds } => {
                        steps.push(Step::WaitForDraw {
                            node_uuid: node.uuid,
                            is_done: rounds.len() == config.num_rounds() as usize,
                            is_first_in_tournament: !has_seen_round_node,
                            previous_break_node: prev_break.clone(),
                        });
                        has_seen_round_node = true;
                        let mut node_is_done = false;
                        if rounds.len() == config.num_rounds() as usize {
                            for round_id in rounds {
                                let round = all_rounds_by_id.get(&round_id).expect("Db constraint failed");
                                steps.push(Step::WaitForMotion { round_uuid: round.uuid, is_done: round.motion.is_some() });
                                if round.motion.is_some() {
                                    steps.push(Step::WaitForPublishRound { round_uuid: round.uuid, is_done: round.draw_release_time.is_some() });
                                    if round.draw_release_time.is_some() {
                                        steps.push(Step::WaitForMotionRelease { round_uuid: round.uuid, is_done: round.full_motion_release_time.is_some() });
                                        if round.full_motion_release_time.is_some() {
                                            let debates = all_debates_by_round_id.get(&round.uuid).unwrap();
                                            let num_scores = debates.iter().filter(|d| d.is_complete).count();

                                            steps.push(Step::WaitForResults { round_uuid: round.uuid, num_submitted: num_scores, num_expected: debates.len(), is_silent: round.is_silent, is_done: round.round_close_time.is_some() });
                                            node_is_done = round.round_close_time.is_some();
                                            if round.round_close_time.is_some() {
                                                continue;
                                            }
                                        }
                                    }
                                }
                                all_nodes_complete = false;
                                node_is_done = false;
                                break;
                            }
                        }
                        else {
                            all_nodes_complete = false;
                            node_is_done = false;
                        }

                        if node_is_done {
                            explore_queue.extend(children.into_iter().map(
                                |u| (*u, prev_break)
                            ));
                        }
                    },
                    open_tab_entities::domain::tournament_plan_node::PlanNodeType::Break { config: _, break_id } => {
                        steps.push(Step::WaitForBreak { node_uuid: node.uuid, is_done: break_id.is_some() });

                        if break_id.is_some() {
                            explore_queue.extend(children.into_iter().map(
                                |u| (*u, Some(node.uuid.clone()))
                            ));
                        }
                        else {
                            all_nodes_complete = false;
                        }
                    },
                }
            }
            if all_nodes_complete {
                steps.push(Step::Done {});
            }    
        }


        Ok(Self {
            steps,
        })
    }

}

#[derive(Serialize)]
#[serde(tag = "step_type")]
enum Step {
    LoadParticipants { is_done: bool },
    WaitForDraw { node_uuid: Uuid, is_done: bool, is_first_in_tournament: bool, previous_break_node: Option<Uuid> },
    WaitForMotion { round_uuid: Uuid, is_done: bool },
    WaitForPublishRound { round_uuid: Uuid, is_done: bool },
    WaitForMotionRelease { round_uuid: Uuid, is_done: bool },
    WaitForResults { round_uuid: Uuid, num_submitted: usize, num_expected: usize, is_silent: bool, is_done: bool },
    WaitForBreak { node_uuid: Uuid, is_done: bool },
    Done {},
}