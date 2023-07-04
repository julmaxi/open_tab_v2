use std::{error::Error};


use base64::{engine::general_purpose, Engine};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::participant::ParticipantInstitution};

use sea_orm::prelude::*;

use crate::{participants_list_view::ParticipantEntry};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateParticipantsAction {
    updated_participants: Vec<ParticipantEntry>,
    tournament_id: Uuid
}

#[async_trait]
impl ActionTrait for UpdateParticipantsAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, Box<dyn Error>> where C: ConnectionTrait {
        let mut groups = EntityGroup::new();

        for participant in self.updated_participants.into_iter() {
            groups.add(Entity::Participant(
                Participant {
                    uuid: participant.uuid,
                    name: participant.name,
                    role: match participant.role {
                        crate::participants_list_view::ParticipantRole::Speaker { team_id } => ParticipantRole::Speaker(Speaker { team_id: Some(team_id) }),
                        crate::participants_list_view::ParticipantRole::Adjudicator { chair_skill, panel_skill } => ParticipantRole::Adjudicator(Adjudicator { chair_skill, panel_skill })
                    },
                    tournament_id: self.tournament_id,
                    institutions: participant.institutions.into_iter().map(|p| ParticipantInstitution {
                        uuid: p.uuid,
                        clash_severity: p.clash_severity as u16
                    }).collect(),
                    registration_key: participant.registration_key.map(|r| general_purpose::STANDARD_NO_PAD.decode(r).map(|r| r[16..48].to_vec())).transpose()?
                }
            ));
        }

        Ok(groups)
    }
}