use std::{error::Error};


use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*};

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
                    institutions: vec![]
                }
            ));
        }

        Ok(groups)
    }
}