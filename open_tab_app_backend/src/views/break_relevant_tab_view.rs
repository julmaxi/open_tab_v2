use std::collections::HashMap;

use itertools::Itertools;
use open_tab_entities::{domain::participant_clash::ParticipantClash, prelude::Participant, EntityGroup, EntityTypeId};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};


use crate::LoadedView;

pub use open_tab_entities::tab::AugmentedBreakRelevantTabView;


pub struct LoadedBreakRelevantTabView {
    node_uuid: Uuid,
    view: AugmentedBreakRelevantTabView
}

impl LoadedBreakRelevantTabView {
    pub async fn load<C>(db: &C, node_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            LoadedBreakRelevantTabView {
                node_uuid,
                view: AugmentedBreakRelevantTabView::load_from_node(db, node_uuid).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedBreakRelevantTabView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_types(vec![
            EntityTypeId::TournamentPlanNode,
            EntityTypeId::TournamentDebate,
            EntityTypeId::TournamentRound,
            EntityTypeId::Participant,
            EntityTypeId::Ballot,
            EntityTypeId::TournamentBreak
        ]) {
            self.view = AugmentedBreakRelevantTabView::load_from_node(db, self.node_uuid).await?;

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
