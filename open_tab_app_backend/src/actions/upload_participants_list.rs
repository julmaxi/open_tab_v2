use std::collections::HashMap;


use itertools::Itertools;
use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{participant::ParticipantInstitution, participant_clash::ParticipantClash}};

use rand::{thread_rng, Rng};
use sea_orm::prelude::*;

use crate::import::{CSVReaderConfig, ParticipantData};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

//use crate::import::;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadParticipantsListAction {
    path: String,
    tournament_id: Uuid,
    parser_config: CSVReaderConfig
}


#[async_trait]
impl ActionTrait for UploadParticipantsListAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut groups = EntityGroup::new();
        let file = std::fs::File::open(self.path.clone())?;
        let parse_result = self.parser_config.parse(&file)?;

        let existing_teams_by_name = open_tab_entities::schema::team::Entity::find()
            .filter(open_tab_entities::schema::team::Column::TournamentId.eq(self.tournament_id))
            .all(db)
            .await?
            .into_iter()
            .map(|team| (team.name.clone(), team))
            .collect::<HashMap<_, _>>();

        let mut all_speakers_with_uuid = Vec::new();
        for team in parse_result.data.teams.into_iter() {
            let team_uuid = existing_teams_by_name.get(&team.name)
                .map(|team| team.uuid.clone())
                .unwrap_or_else(Uuid::new_v4);

            let team_entity = open_tab_entities::domain::team::Team {
                uuid: team_uuid,
                name: team.name,
                tournament_id: self.tournament_id,
            };
            groups.add(Entity::Team(team_entity));

            for member in team.members.into_iter() {
                all_speakers_with_uuid.push(
                    (
                        member,
                        ParticipantRole::Speaker(
                            Speaker {
                                team_id: Some(team_uuid),
                            }
                        ),
                        Uuid::new_v4(),
                    )
                )
            }
        }

        let all_adjudicators_with_uuid = parse_result.data.adjudicators.into_iter()
        .map(|adjudicator| (
            adjudicator,
            ParticipantRole::Adjudicator(
                Adjudicator {
                    chair_skill: 0,
                    panel_skill: 0,
                    unavailable_rounds: Vec::new(),
                }
            ),
            Uuid::new_v4()
        )).collect::<Vec<_>>();

        let all_participants_with_uuid = all_speakers_with_uuid.iter().map(|(speaker, role, uuid)| (&speaker.participant_data, role.clone(), uuid.clone()))
        .chain(all_adjudicators_with_uuid.iter().map(|(adjudicator, role, uuid)| (&adjudicator.participant_data, role.clone(), uuid.clone()))).collect_vec();

        let participant_entities = Self::get_participants(&self, all_participants_with_uuid, db).await?;

        for participant_entity in participant_entities {
            groups.add(participant_entity);
        }

        Ok(groups)
    }
}

impl UploadParticipantsListAction {
    async fn get_participants<C>(&self, participants_with_uuid_and_role: Vec<(&ParticipantData, ParticipantRole, Uuid)>, db: &C) -> Result<
        Vec<Entity>, anyhow::Error> where C: sea_orm::ConnectionTrait
     {
        let mut out_entities = Vec::new();
        let mut existing_institution_uuids_by_name = open_tab_entities::schema::tournament_institution::Entity::find()
            .filter(open_tab_entities::schema::tournament_institution::Column::TournamentId.eq(self.tournament_id))
            .all(db)
            .await?
            .into_iter()
            .map(|i| (i.name.clone(), i.uuid))
            .collect::<HashMap<_, _>>();

        let mut participant_uuids_by_name = open_tab_entities::schema::participant::Entity::find()
            .filter(open_tab_entities::schema::participant::Column::TournamentId.eq(self.tournament_id))
            .all(db)
            .await?
            .into_iter()
            .map(|p| (p.name.clone(), p.uuid))
            .collect::<HashMap<_, _>>();

        participant_uuids_by_name.extend(
            participants_with_uuid_and_role.iter().map(|o| (o.0.name.clone(), o.2))
        );

        for (participant, role, uuid) in participants_with_uuid_and_role {
            let institutions = participant.institutions.iter().map(|institution_name| {
                    let institution_uuid = existing_institution_uuids_by_name.get(institution_name)
                    .map(|uuid| uuid.clone())
                    .unwrap_or_else(|| {
                        let uuid = Uuid::new_v4();

                        out_entities.push(
                            Entity::TournamentInstitution(
                                open_tab_entities::domain::tournament_institution::TournamentInstitution {
                                    uuid: uuid.clone(),
                                    name: institution_name.clone(),
                                    tournament_id: self.tournament_id,
                                }
                            )
                        );

                        existing_institution_uuids_by_name.insert(institution_name.clone(), uuid);
                        uuid
                    });
                    ParticipantInstitution {
                        uuid: institution_uuid,
                        clash_severity: 100
                    }
                }).collect::<Vec<_>>();

            let registration_key : [u8; 32] = thread_rng().gen();
 
            let out_participant_entity = open_tab_entities::domain::participant::Participant {
                uuid: uuid,
                name: participant.name.clone(),
                role,
                institutions,
                tournament_id: self.tournament_id,
                registration_key: Some(registration_key.to_vec())
            };

            out_entities.push(Entity::Participant(out_participant_entity));

            for clash in participant.clashes.iter() {
                let clash_uuid = Uuid::new_v4();
                let clash_participant_uuid = participant_uuids_by_name.get(clash)
                    .map(|uuid| uuid.clone());

                if let Some(target_participant_id) = clash_participant_uuid {
                    out_entities.push(
                        Entity::ParticipantClash(ParticipantClash {
                            uuid: clash_uuid,
                            declaring_participant_id: uuid,
                            target_participant_id,
                            clash_severity: 100,
                        }
                    ));
                }
            }
        }

        Ok(out_entities)
    }
}
