


use base64::{engine::general_purpose, Engine};
use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{participant::ParticipantInstitution, participant_clash::ParticipantClash, self}};

use sea_orm::prelude::*;

use crate::participants_list_view::{ParticipantEntry, ParticipantTeamInfo};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

use open_tab_entities::group::EntityType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateParticipantsAction {
    updated_participants: Vec<ParticipantEntry>,
    #[serde(default)]
    deleted_participants: Vec<Uuid>,
    tournament_id: Uuid
}

#[async_trait]
impl ActionTrait for UpdateParticipantsAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut groups = EntityGroup::new();

        for participant in self.updated_participants.into_iter() {
            if self.deleted_participants.contains(&participant.uuid) {
                continue;
            }
            let existing_clashes = open_tab_entities::schema::participant_clash::Entity::find()
                .filter(open_tab_entities::schema::participant_clash::Column::DeclaringParticipantId.eq(participant.uuid))
                .all(_db)
                .await?;

            let new_clashes = participant.clashes.into_iter().filter(
                |c| c.clash_direction == crate::participants_list_view::ClashDirection::Outgoing
            ).map(|c| (c.participant_uuid, c.clash_severity)).collect::<Vec<_>>();

            let new_uuids = new_clashes.iter().map(|(uuid, _)| uuid.clone()).collect::<Vec<_>>();

            let to_delete_ids = existing_clashes.iter().filter(|c| !new_uuids.contains(&c.uuid)).map(|c| c.uuid).collect::<Vec<_>>();
            to_delete_ids.iter().for_each(|id| {
                groups.delete(EntityType::ParticipantClash, id.clone());
            });

            new_clashes.into_iter().for_each(
                |(uuid, clash_severity)| {
                    groups.add(Entity::ParticipantClash(
                        ParticipantClash {
                            uuid: uuid.clone(),
                            declaring_participant_id: participant.uuid,
                            target_participant_id: uuid,
                            clash_severity: clash_severity as u16
                        }
                    ));
                }
            );
        
            let mut participant_role = participant.role;

            let old_speaker = open_tab_entities::schema::speaker::Entity::find()
            .filter(open_tab_entities::schema::speaker::Column::Uuid.eq(participant.uuid))
            .one(_db)
            .await?;

            //TODO: Could be one query with some work
            if let Some(old_speaker) = old_speaker {
                let did_remove_from_team = match &participant_role {
                    crate::participants_list_view::ParticipantRole::Speaker { team_info } => {
                        match &team_info {
                            ParticipantTeamInfo::Existing { team_id } if Some(*team_id) == old_speaker.team_id => false,
                            ParticipantTeamInfo::New { .. } | ParticipantTeamInfo::Existing { .. } => {
                                true
                            },
                        }
                    },
                    crate::participants_list_view::ParticipantRole::Adjudicator { .. } => true
                };

                if did_remove_from_team {
                    let old_team_members = open_tab_entities::schema::speaker::Entity::find()
                    .filter(open_tab_entities::schema::speaker::Column::TeamId.eq(old_speaker.team_id))
                    .count(_db)
                    .await?;

                    if old_team_members == 1 {
                        groups.delete(EntityType::Team, old_speaker.team_id.unwrap());
                    }
                }
            }


            if let crate::participants_list_view::ParticipantRole::Speaker { team_info } = &mut participant_role {
                match team_info {
                    ParticipantTeamInfo::New { new_team_name } => {
                        let new_team = domain::team::Team {
                            uuid: Uuid::new_v4(),
                            name: new_team_name.clone(),
                            tournament_id: self.tournament_id
                        };
                        let new_uuid = new_team.uuid;
                        *team_info = ParticipantTeamInfo::Existing { team_id: new_team.uuid };
                        groups.add(Entity::Team(new_team));
                        new_uuid
                    },
                    ParticipantTeamInfo::Existing { team_id } => {
                        team_id.clone()
                    }
                };
            }

            let role = match participant_role {
                crate::participants_list_view::ParticipantRole::Speaker { team_info: ParticipantTeamInfo::Existing { team_id } } => ParticipantRole::Speaker(Speaker { team_id: Some(team_id) }),
                crate::participants_list_view::ParticipantRole::Adjudicator { chair_skill, panel_skill, unavailable_rounds } => ParticipantRole::Adjudicator(Adjudicator { chair_skill, panel_skill, unavailable_rounds }),
                _ => unreachable!("Should not be possible to have a new team here")
            };

            groups.add(Entity::Participant(
                Participant {
                    uuid: participant.uuid,
                    name: participant.name,
                    role,
                    tournament_id: self.tournament_id,
                    institutions: participant.institutions.into_iter().map(|p| ParticipantInstitution {
                        uuid: p.uuid,
                        clash_severity: p.clash_severity as u16
                    }).collect(),
                    registration_key: participant.registration_key.map(|r| general_purpose::URL_SAFE_NO_PAD.decode(r).map(|r| r[16..48].to_vec())).transpose()?,
                    is_anonymous: participant.is_anonymous
                }
            ));
        }

        for uuid in self.deleted_participants.into_iter() {
            groups.delete(EntityType::Participant, uuid);
        }

        Ok(groups)
    }
}