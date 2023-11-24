use std::collections::HashMap;

use open_tab_entities::EntityGroup;
use sea_orm::{prelude::Uuid, ConnectionTrait};


use crate::LoadedView;

pub use open_tab_entities::tab::BreakRelevantTabView;


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
        if changes.tournament_plan_nodes.len() > 0 || changes.tournament_debates.len() > 0 || changes.tournament_rounds.len() > 0 || changes.participants.len() > 0 || changes.ballots.len() > 0 || changes.tournament_breaks.len() > 0 {
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
