use std::collections::HashMap;

use itertools::Itertools;
use open_tab_entities::{domain::{clash_declaration::ClashDeclaration, institution_declaration, participant_clash::ParticipantClash, tournament_institution}, prelude::Participant, schema, EntityGroup, EntityTypeId};
use open_tab_server::sync::FatLog;
use sea_orm::{prelude::*, QuerySelect};
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
            EntityTypeId::ClashDeclaration,
            EntityTypeId::InstitutionDeclaration,
            EntityTypeId::TournamentInstitution,
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
#[serde(tag = "type")]
pub enum ClashTarget {
    Participant { uuid: Uuid },
    Institution { uuid: Uuid },
}

#[derive(Serialize, Deserialize)]
pub struct ClashDeclarationInfo {
    declaration_id: Uuid,
    pub declaring_participant_name: String,
    pub declaring_participant_uuid: Uuid,
    pub target_name: String,
    #[serde(flatten)]
    pub target: ClashTarget,
    pub clash_state: ClashState,
    pub is_retracted: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ClashesView {
    pending_clashes: Vec<ClashDeclarationInfo>,
    approved_clashes: Vec<ClashDeclarationInfo>,
    rejected_clashes: Vec<ClashDeclarationInfo>,
}

impl ClashesView {
    async fn load_from_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let clashes = ParticipantClash::get_all_in_tournament(db, tournament_id).await?;
        let participant_institutions = schema::participant_tournament_institution::Entity::find()
            .inner_join(schema::participant::Entity)
            .filter(schema::participant::Column::TournamentId.eq(tournament_id))
            .all(db)
            .await?;
        let declarations = ClashDeclaration::get_all_in_tournament(db, tournament_id).await?;
        let institution_declarations = institution_declaration::InstitutionDeclaration::get_all_in_tournament(db, tournament_id).await?;
        
        let all_participants = Participant::get_all_in_tournament(db, tournament_id).await?;
        let participant_names = all_participants.into_iter().map(|p| (p.uuid, p.name)).collect::<HashMap<Uuid, String>>();

        let all_institutions = tournament_institution::TournamentInstitution::get_all_in_tournament(db, tournament_id).await?;
        let institution_names = all_institutions.into_iter().map(|i| (i.uuid, i.name)).collect::<HashMap<Uuid, String>>();

        let clashes_by_participants = clashes.into_iter().map(|c| ((c.declaring_participant_id, c.target_participant_id), c)).collect::<HashMap<(Uuid, Uuid), ParticipantClash>>();
        let institution_by_participants = participant_institutions.into_iter().map(|c| ((c.participant_id, c.institution_id), c)).collect::<HashMap<_, _>>();

        let mut clash_groups = declarations.into_iter().map(
            |d: ClashDeclaration| {
                let has_clash = clashes_by_participants.contains_key(&(d.source_participant_id, d.target_participant_id));

                let c = ClashDeclarationInfo {
                    declaration_id: d.uuid,
                    declaring_participant_name: participant_names.get(&d.source_participant_id).unwrap_or(&"Unknown".to_string()).clone(),
                    declaring_participant_uuid: d.source_participant_id,
                    target_name: participant_names.get(&d.target_participant_id).unwrap_or(&"Unknown".to_string()).clone(),
                    target: ClashTarget::Participant { uuid: d.target_participant_id },
                    clash_state: match (d.was_seen, has_clash, d.is_retracted) {
                        (_, a, b) if a != b => ClashState::Approved,
                        (true, _, _) => ClashState::Rejected,
                        (false, _, _) => ClashState::Pending,
                    },
                    is_retracted: d.is_retracted,
                };

                c
            }
        ).into_group_map_by(|c| c.clash_state);

        let mut institution_groups = institution_declarations.into_iter().map(
            |d: institution_declaration::InstitutionDeclaration| {
                let has_clash = institution_by_participants.contains_key(&(d.source_participant_id, d.tournament_institution_id));

                let c = ClashDeclarationInfo {
                    declaration_id: d.uuid,
                    declaring_participant_name: participant_names.get(&d.source_participant_id).unwrap_or(&"Unknown".to_string()).clone(),
                    declaring_participant_uuid: d.source_participant_id,
                    target_name: institution_names.get(&d.tournament_institution_id).unwrap_or(&"Unknown".to_string()).clone(),
                    target: ClashTarget::Institution { uuid: d.tournament_institution_id },
                    clash_state: match (d.was_seen, has_clash, d.is_retracted) {
                        (_, a, b) if a != b => ClashState::Approved,
                        (true, _, _) => ClashState::Rejected,
                        (false, _, _) => ClashState::Pending,
                    },
                    is_retracted: d.is_retracted,
                };

                c
            }
        ).into_group_map_by(|c| c.clash_state);

        return Ok(
            ClashesView {
                pending_clashes: clash_groups.remove(&ClashState::Pending).unwrap_or_default().into_iter().chain(institution_groups.remove(&ClashState::Pending).unwrap_or_default().into_iter()).collect(),
                approved_clashes: clash_groups.remove(&ClashState::Approved).unwrap_or_default().into_iter().chain(institution_groups.remove(&ClashState::Approved).unwrap_or_default().into_iter()).collect(),
                rejected_clashes: clash_groups.remove(&ClashState::Rejected).unwrap_or_default().into_iter().chain(institution_groups.remove(&ClashState::Rejected).unwrap_or_default().into_iter()).collect(),
            }
        )
        /*let all_clashes = ParticipantClash::get_all_in_tournament(db, tournament_id).await?;
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
        )*/
    }
}