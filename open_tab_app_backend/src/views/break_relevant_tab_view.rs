use std::collections::HashMap;

use open_tab_entities::{tab::TabView, domain::entity::LoadEntity, info::TournamentParticipantsInfo, EntityGroup};
use sea_orm::{prelude::Uuid, ConnectionTrait};
use serde::Serialize;

use crate::{views, LoadedView};



pub struct LoadedBreakRelevantTabView {
    node_uuid: Uuid,
    view: BreakRelevantTabView
}

impl LoadedBreakRelevantTabView {
    pub async fn load<C>(db: &C, node_uuid: Uuid) -> Result<Self, anyhow::Error> where C: ConnectionTrait {
        Ok(
            LoadedBreakRelevantTabView {
                node_uuid,
                view: BreakRelevantTabView::load_from_node(db, node_uuid).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedBreakRelevantTabView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.tournament_plan_nodes.len() > 0 || changes.tournament_debates.len() > 0 || changes.tournament_rounds.len() > 0 || changes.participants.len() > 0 || changes.ballots.len() > 0 {
            self.view = BreakRelevantTabView::load_from_node(db, self.node_uuid).await?;

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
pub struct BreakRelevantTabView {
    tab: TabView,
    speaker_teams: HashMap<Uuid, Uuid>,
    team_members: HashMap<Uuid, Vec<Uuid>>
}

impl BreakRelevantTabView {
    async fn load_from_node<C>(db: &C, node_uuid: Uuid) -> Result<BreakRelevantTabView, anyhow::Error> where C: ConnectionTrait {
        let target_node = open_tab_entities::domain::tournament_plan_node::TournamentPlanNode::get(db, node_uuid).await?;
        let break_background = crate::actions::execute_plan_node::BreakNodeBackgroundInfo::load_for_break_node(db, target_node.tournament_id, node_uuid).await?;
        let speaker_info = TournamentParticipantsInfo::load(db, target_node.tournament_id).await?;

        let tab = views::tab_view::TabView::load_from_rounds(
            db,
            break_background.preceding_rounds.clone(),
            &speaker_info
        ).await?;

        Ok(BreakRelevantTabView {
            tab,
            speaker_teams: speaker_info.speaker_teams,
            team_members: speaker_info.team_members
        })
    }
}