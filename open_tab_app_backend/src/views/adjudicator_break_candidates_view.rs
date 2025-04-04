use std::{collections::HashMap};

use itertools::Itertools;
use open_tab_entities::{EntityGroup, derived_models::BreakNodeBackgroundInfo, domain::entity::LoadEntity, EntityTypeId};
use sea_orm::{prelude::Uuid, EntityTrait, QueryFilter, ColumnTrait};
use serde::Serialize;


use crate::{draw::{clashes::ClashType, evaluation::{DrawEvaluator, TournamentObservingDrawEvaluationContext}}, LoadedView};


pub struct LoadedAdjudicatorBreakCandidatesView {
    node_uuid: Uuid,
    view: AdjudicatorBreakCandidatesView
}

impl LoadedAdjudicatorBreakCandidatesView {
    pub async fn load<C>(db: &C, node_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            LoadedAdjudicatorBreakCandidatesView {
                node_uuid,
                view: AdjudicatorBreakCandidatesView::load_from_node(db, node_uuid).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedAdjudicatorBreakCandidatesView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_types(vec![EntityTypeId::ParticipantClash, EntityTypeId::TournamentPlanNode, EntityTypeId::Participant]) {
            self.view = AdjudicatorBreakCandidatesView::load_from_node(db, self.node_uuid).await?;

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

#[derive(Serialize)]
struct AdjudicatorBreakCandidatesView {
    adjudicators: Vec<AdjudicatorBreakInfo>
}

#[derive(Serialize)]
enum ClashState {
    NoClashes,
    SomeClashes,
    FullyClashed,
}

#[derive(Serialize)]
struct AdjudicatorBreakInfo {
    name: String,
    uuid: Uuid,
    clash_state: ClashState,
    is_in_previous_break: bool,
}

impl AdjudicatorBreakCandidatesView {
    async fn load_from_node<C>(db: &C, node_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut out = AdjudicatorBreakCandidatesView {
            adjudicators: vec![]
        };
        let target_node = open_tab_entities::domain::tournament_plan_node::TournamentPlanNode::get(db, node_uuid).await?;
        let adjudicators = open_tab_entities::domain::participant::Participant::get_all_in_tournament(db, target_node.tournament_id).await?;
        let adjudicators = adjudicators.into_iter().filter(|adj| matches!(
            adj.role,
            open_tab_entities::domain::participant::ParticipantRole::Adjudicator {..}
        )).collect::<Vec<_>>();

        let team_members = open_tab_entities::schema::speaker::Entity::find().inner_join(
            open_tab_entities::schema::participant::Entity
        ).filter(
            open_tab_entities::schema::participant::Column::TournamentId.eq(target_node.tournament_id)
        ).all(db).await?.into_iter().filter_map(|speaker| speaker.team_id.map(|tid| (tid, speaker.uuid))).into_group_map();

        let break_background = BreakNodeBackgroundInfo::load_for_break_node(db, target_node.tournament_id, node_uuid).await?;

        let previous_breaking_adjudicators = if let Some(Some(prev_break)) = break_background.relevant_break_id {
            let break_ = open_tab_entities::domain::tournament_break::TournamentBreak::get(db, prev_break).await?;

            let breaking_adjs = break_.breaking_adjudicators;
            if breaking_adjs.len() == 0 {
                adjudicators.iter().map(|adjudicator| adjudicator.uuid).collect::<Vec<_>>()
            }
            else {
                breaking_adjs
            }            
        }
        else {
            adjudicators.iter().map(|adjudicator| adjudicator.uuid).collect::<Vec<_>>()
        };

        let (breaking_teams, breaking_speakers) = match &target_node.config {
            open_tab_entities::domain::tournament_plan_node::PlanNodeType::Break { config: _, break_id: Some(break_id), .. } => {
                let break_ = open_tab_entities::domain::tournament_break::TournamentBreak::get(db, *break_id).await?;

                let breaking_teams = break_.breaking_teams;
                let breaking_speakers = break_.breaking_speakers;

                (breaking_teams, breaking_speakers)
            },
            _ => (vec![], vec![])
        };

        let evaluation_context = TournamentObservingDrawEvaluationContext::new_from_tournament(db, target_node.tournament_id).await?;
        let evaluator = DrawEvaluator::new(
            Default::default(),
            vec![],
            &evaluation_context
        );

        for adj in adjudicators.into_iter() {
            let mut num_clashing_teams = 0;
            let mut num_clashing_speakers = 0;

            for team in breaking_teams.iter() {
                let empty = vec![];
                let members = team_members.get(team).unwrap_or(&empty);
                let has_clash = members.iter().any(|member| {
                    let clashes = evaluator.get_permanent_clashes_between_participants(adj.uuid, *member);
                    clashes.iter().any(|c| match &c {
                        ClashType::DeclaredClash{severity} => *severity > 0,
                        ClashType::InstitutionalClash{severity, ..} => *severity > 0,
                        _ => false
                    })
                });
                if has_clash {
                    num_clashing_teams += 1;
                }
            }

            for speaker in breaking_speakers.iter() {
                let clashes = evaluator.get_permanent_clashes_between_participants(adj.uuid, *speaker);
                if clashes.iter().any(|c| match &c {
                    ClashType::DeclaredClash{severity} => *severity > 0,
                    ClashType::InstitutionalClash{severity, ..} => *severity > 0,
                    _ => false
                }) {
                    num_clashing_speakers += 1;
                }
            }

            let clash_state = match (num_clashing_teams, num_clashing_speakers) {
                (0, 0) => ClashState::NoClashes,
                (num_clashing_teams, _) if num_clashing_teams == breaking_teams.len() => ClashState::FullyClashed,
                (_, _) => ClashState::SomeClashes,
            };

            let is_in_previous_break = previous_breaking_adjudicators.contains(&adj.uuid);

            out.adjudicators.push(AdjudicatorBreakInfo {
                name: adj.name.clone(),
                uuid: adj.uuid,
                clash_state,
                is_in_previous_break
            });
        }

        Ok(out)
    }
}