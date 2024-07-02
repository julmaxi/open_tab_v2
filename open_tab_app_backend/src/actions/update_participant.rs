


use std::collections::HashMap;

use base64::{engine::general_purpose, Engine};
use async_trait::async_trait;
use open_tab_entities::{domain::{self, entity::LoadEntity, participant::ParticipantInstitution, participant_clash::ParticipantClash, team}, prelude::*};

use rand::{thread_rng, Rng};
use sea_orm::{prelude::*, FromQueryResult, QuerySelect, SelectColumns};

use crate::participants_list_view::{ParticipantEntry, ParticipantTeamInfo};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

use open_tab_entities::group::EntityTypeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateParticipantsAction {
    #[serde(default)]
    updated_participants: Vec<ParticipantEntry>,
    #[serde(default)]
    added_participants: Vec<ParticipantEntry>,
    #[serde(default)]
    deleted_participants: Vec<Uuid>,
    tournament_id: Uuid
}

#[async_trait]
impl ActionTrait for UpdateParticipantsAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut groups = EntityGroup::new(self.tournament_id);

        let mut team_member_count_changes : HashMap<Uuid, i32> = HashMap::new();

        let mut new_teams_created = HashMap::<String, Uuid>::new();

        for participant in self.added_participants.into_iter() {
            let mut participant_role = participant.role;
            if let crate::participants_list_view::ParticipantRole::Speaker { team_info } = &mut participant_role {
                match team_info {
                    ParticipantTeamInfo::New { new_team_name } => {
                        let team_id = if let Some(team_id) = new_teams_created.get(new_team_name) {
                            *team_id
                        }
                        else {
                            let new_team = domain::team::Team {
                                uuid: Uuid::new_v4(),
                                name: new_team_name.clone(),
                                tournament_id: self.tournament_id
                            };
                            let new_uuid = new_team.uuid;
                            new_teams_created.insert(new_team_name.clone(), new_uuid);
                            groups.add(Entity::Team(new_team));
                            new_uuid
                        };
                        
                        *team_info = ParticipantTeamInfo::Existing { team_id };
                        team_id
                    },
                    ParticipantTeamInfo::Existing { team_id } => {
                        team_member_count_changes.entry(*team_id).and_modify(|c| *c += 1).or_insert(1);

                        team_id.clone()
                    }
                };
            }

            let role = match participant_role {
                crate::participants_list_view::ParticipantRole::Speaker { team_info: ParticipantTeamInfo::Existing { team_id } } => ParticipantRole::Speaker(Speaker { team_id: Some(team_id) }),
                crate::participants_list_view::ParticipantRole::Adjudicator { chair_skill, panel_skill, unavailable_rounds } => ParticipantRole::Adjudicator(Adjudicator { chair_skill, panel_skill, unavailable_rounds }),
                _ => unreachable!("Should not be possible to have a new team here")
            };

            let registration_key : [u8; 32] = thread_rng().gen();
            groups.add(Entity::Participant(
                Participant {
                    uuid: if participant.uuid.is_nil() { Uuid::new_v4() } else { participant.uuid },
                    name: participant.name,
                    role,
                    tournament_id: self.tournament_id,
                    institutions: participant.institutions.into_iter().map(|p| ParticipantInstitution {
                        uuid: p.uuid,
                        clash_severity: p.clash_severity as u16
                    }).collect(),
                    registration_key: Some(registration_key.to_vec()),
                    is_anonymous: participant.is_anonymous,
                    user_id: participant.user_id
                }
            ));
        }

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
            ).map(|c| (Uuid::new_v4(), c.participant_uuid, c.clash_severity)).collect::<Vec<_>>();

            let new_uuids = new_clashes.iter().map(|(uuid, _, _)| uuid.clone()).collect::<Vec<_>>();

            let to_delete_ids = existing_clashes.iter().filter(|c| !new_uuids.contains(&c.uuid)).map(|c| c.uuid).collect::<Vec<_>>();
            to_delete_ids.iter().for_each(|id| {
                groups.delete(EntityTypeId::ParticipantClash, id.clone());
            });

            new_clashes.into_iter().for_each(
                |(uuid, target_uuid, clash_severity)| {
                    groups.add(Entity::ParticipantClash(
                        ParticipantClash {
                            uuid: uuid,
                            declaring_participant_id: participant.uuid,
                            target_participant_id: target_uuid,
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

            if let Some(old_speaker) = old_speaker {
                if let Some(team_id) = old_speaker.team_id {
                    team_member_count_changes.entry(team_id).and_modify(|c| *c -= 1).or_insert(-1);
                }
            }


            if let crate::participants_list_view::ParticipantRole::Speaker { team_info } = &mut participant_role {
                match team_info {
                    ParticipantTeamInfo::New { new_team_name } => {
                        let team_id = if let Some(team_id) = new_teams_created.get(new_team_name) {
                            *team_id
                        }
                        else {
                            let new_team = domain::team::Team {
                                uuid: Uuid::new_v4(),
                                name: new_team_name.clone(),
                                tournament_id: self.tournament_id
                            };
                            let new_uuid = new_team.uuid;    
                            new_teams_created.insert(new_team_name.clone(), new_uuid);
                            groups.add(Entity::Team(new_team));
                            new_uuid
                        };
                        
                        *team_info = ParticipantTeamInfo::Existing { team_id };
                        team_id
                    },
                    ParticipantTeamInfo::Existing { team_id } => {
                        team_member_count_changes.entry(*team_id).and_modify(|c| *c += 1).or_insert(1);

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
                    is_anonymous: participant.is_anonymous,
                    user_id: participant.user_id
                }
            ));
        }

        let deleted_participant_models = open_tab_entities::domain::participant::Participant::get_many(_db, self.deleted_participants.clone()).await?;     
        for participant in deleted_participant_models {
            groups.delete(EntityTypeId::Participant, participant.uuid);
            match participant.role {
                ParticipantRole::Speaker(speaker) => {
                    if let Some(team_id) = speaker.team_id {
                        team_member_count_changes.entry(team_id).and_modify(|c| *c -= 1).or_insert(-1);
                    }
                },
                _ => {}
            }
        }

        #[derive(Debug, FromQueryResult)]
        struct TeamMemberCount {
            team_id: Uuid,
            count: i32,
        }

        let possibly_empty_teams = team_member_count_changes.iter().filter(|(_, c)| **c < 0).map(|(k, _)| *k).collect::<Vec<_>>();
        
        let changed_team_member_count = open_tab_entities::schema::speaker::Entity::find()
            .select_only()
            .select_column(open_tab_entities::schema::speaker::Column::TeamId)
            .select_column_as(open_tab_entities::schema::speaker::Column::Uuid.count(), "count")
            .group_by(open_tab_entities::schema::speaker::Column::TeamId)
            .filter(open_tab_entities::schema::speaker::Column::TeamId.is_in(possibly_empty_teams))
            .into_model::<TeamMemberCount>()
            .all(_db)
            .await?;

        let deleted_teams = changed_team_member_count.iter().filter(|c| c.count + team_member_count_changes.get(&c.team_id).copied().unwrap_or(0) <= 0).map(|c| c.team_id).collect::<Vec<_>>();

        for team_id in deleted_teams {
            groups.delete(EntityTypeId::Team, team_id);
        }

        Ok(groups)
    }
}