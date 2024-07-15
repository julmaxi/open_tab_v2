use itertools::Itertools;
use sea_orm::prelude::Uuid;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::{prelude::*, QueryOrder};
use open_tab_entities::{prelude::*, domain, EntityTypeId};

use open_tab_entities::schema::{self, tournament_round};




use crate::LoadedView;
use crate::tournament_tree_view::get_round_names;

pub struct LoadedRoundsView {
    pub view: RoundsView,
    pub tournament_id: Uuid
}

impl LoadedRoundsView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<LoadedRoundsView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            LoadedRoundsView {
                tournament_id: tournament_uuid,
                view: RoundsView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedRoundsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_types(vec![
            EntityTypeId::TournamentPlanNode,
            EntityTypeId::TournamentPlanEdge,
            EntityTypeId::TournamentRound
        ]) {
            self.view = RoundsView::load_from_tournament(db, self.tournament_id).await?;

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
pub struct RoundsView {
    rounds: Vec<RoundOverview>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundOverview {
    uuid: Uuid,
    round_number: i32,
    name: String,
}


impl RoundsView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<RoundsView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(
            tournament_round::Column::TournamentId.eq(tournament_uuid)
        ).order_by_asc(tournament_round::Column::Index).all(db).await?;

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
        let names = get_round_names(&nodes, &node_children, &nodes.iter().filter_map(
            |n| {
                if nodes_to_parents.contains_key(&n.uuid) {
                    None
                }
                else {
                    Some(n.uuid)
                }
            }
        ).collect::<Vec<_>>())?;

        let round_overviews = rounds.into_iter().map(|round| {
            RoundOverview {
                uuid: round.uuid,
                round_number: round.index,
                name: names.by_round_ids.get(&round.uuid).cloned().unwrap_or_else(|| format!("Round {}", round.index + 1)),
            }
        }).collect();

        Ok(
            RoundsView {
                rounds: round_overviews
            }
        )
    }
}