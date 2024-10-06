use std::collections::HashMap;

use itertools::Itertools;
use open_tab_entities::{domain::participant_clash::ParticipantClash, prelude::Participant, EntityGroup, EntityTypeId};
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};


use crate::LoadedView;


pub struct LoadedClashesView {
    tournament_id: Uuid,
    view: ClashesView
}

impl LoadedClashesView {
    pub async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            LoadedClashesView {
                tournament_id,
                view: ClashesView::load_from_tournament(db, tournament_id).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedClashesView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_types(vec![
            EntityTypeId::ParticipantClash,
            EntityTypeId::Participant,
        ]) {
            self.view = ClashesView::load_from_tournament(db, self.tournament_id).await?;

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


#[derive(Debug,Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClashState {
    Approved,
    Rejected,
    Pending,
}

#[derive(Serialize, Deserialize)]
pub struct ClashInfo {
    clash_id: Uuid,
    pub declaring_participant_name: String,
    pub declaring_participant_uuid: Uuid,
    pub target_participant_name: String,
    pub target_participant_uuid: Uuid,
    pub clash_state: ClashState,
    pub is_user_declared: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ClashesView {
    pending_clashes: Vec<ClashInfo>,
    approved_clashes: Vec<ClashInfo>,
    rejected_clashes: Vec<ClashInfo>,
}

impl ClashesView {
    async fn load_from_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let all_clashes = ParticipantClash::get_all_in_tournament(db, tournament_id).await?;
        let all_participants = Participant::get_all_in_tournament(db, tournament_id).await?;
        let participant_names = all_participants.into_iter().map(|p| (p.uuid, p.name)).collect::<HashMap<Uuid, String>>();

        let mut clashes = all_clashes.into_iter().map(
            |c| ClashInfo {
                clash_id: c.uuid,
                declaring_participant_name: participant_names.get(&c.declaring_participant_id).unwrap_or(&"Unknown".to_string()).clone(),
                declaring_participant_uuid: c.declaring_participant_id,
                target_participant_name: participant_names.get(&c.target_participant_id).unwrap_or(&"Unknown".to_string()).clone(),
                target_participant_uuid: c.target_participant_id,
                clash_state: match (c.is_approved, c.was_seen) {
                    (true, _) => ClashState::Approved,
                    (false, true) => ClashState::Rejected,
                    (false, false) => ClashState::Pending,
                },
                is_user_declared: c.is_user_declared,
            }
        ).into_group_map_by(|c| c.clash_state);

        return Ok(
            ClashesView {
                pending_clashes: clashes.remove(&ClashState::Pending).unwrap_or_default(),
                approved_clashes: clashes.remove(&ClashState::Approved).unwrap_or_default(),
                rejected_clashes: clashes.remove(&ClashState::Rejected).unwrap_or_default(),
            }
        )
    }
}