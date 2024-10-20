

use itertools::Itertools;
use open_tab_entities::{domain::{self, entity::LoadEntity, participant::{self, ParticipantInstitution}}, schema, Entity, EntityGroup, EntityTypeId};
use sea_orm::prelude::*;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;

use open_tab_entities::domain::participant_clash::ParticipantClash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateClashes {
    pub tournament_id: Uuid,
    #[serde(default)]
    pub clash_read_declarations: Vec<Uuid>,
    #[serde(default)]
    pub institution_read_declarations: Vec<Uuid>,
    #[serde(default)]
    pub deleted_participant_clashes: Vec<(Uuid, Uuid)>,
    #[serde(default)]
    pub added_participant_clashes: Vec<(Uuid, Uuid)>,
    #[serde(default)]
    pub deleted_institution_clashes: Vec<(Uuid, Uuid)>,
    #[serde(default)]
    pub added_institution_clashes: Vec<(Uuid, Uuid)>,
}


#[async_trait]
impl ActionTrait for UpdateClashes {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new(self.tournament_id);

        if !self.clash_read_declarations.is_empty() {
            let declarations = domain::clash_declaration::ClashDeclaration::get_many(db, self.clash_read_declarations).await?;
            for mut declaration in declarations {
                declaration.was_seen = true;
                g.add(
                    Entity::ClashDeclaration(declaration)
                );
            }
        }

        if !self.institution_read_declarations.is_empty() {
            let declarations = domain::institution_declaration::InstitutionDeclaration::get_many(db, self.institution_read_declarations).await?;
            for mut declaration in declarations {
                declaration.was_seen = true;
                g.add(
                    Entity::InstitutionDeclaration(declaration)
                );
            }
        }

        let clash_declaring_participant_ids = self.added_participant_clashes.iter()
            .map(|(a, _)| *a)
            .chain(self.deleted_participant_clashes.iter().map(|(a, _)| *a))
            .collect::<Vec<_>>();
        let possibly_relevant_clashes = schema::participant_clash::Entity::find()
            .filter(schema::participant_clash::Column::DeclaringParticipantId.is_in(clash_declaring_participant_ids))
            .all(db)
            .await?
            .into_iter()
            .into_group_map_by(|c| (c.declaring_participant_id, c.target_participant_id));
        for (declaring_participant_id, target_participant_id) in self.added_participant_clashes {
            if !possibly_relevant_clashes.contains_key(&(declaring_participant_id, target_participant_id)) {
                let new_clash = ParticipantClash {
                    uuid: Uuid::new_v4(),
                    declaring_participant_id,
                    target_participant_id,
                    clash_severity: 100,
                };
                g.add(
                    Entity::ParticipantClash(new_clash)
                );
            }
        }

        for (declaring_participant_id, target_participant_id) in self.deleted_participant_clashes {
            for clash in possibly_relevant_clashes.get(&(declaring_participant_id, target_participant_id)).into_iter().flatten() {
                g.delete(
                    EntityTypeId::ParticipantClash,
                    clash.uuid
                );
            }
        }

        let institution_declaring_participant_ids = self.added_institution_clashes.iter()
            .map(|(a, _)| *a)
            .chain(self.deleted_institution_clashes.iter().map(|(a, _)| *a))
            .collect::<Vec<_>>();

        let mut relevant_participants = domain::participant::Participant::get_many(db, institution_declaring_participant_ids).await?.into_iter().map(|p| (p.uuid, p)).collect::<std::collections::HashMap<_, _>>();

        for (participant_id, institution_id) in self.added_institution_clashes {
            if let Some(participant) = relevant_participants.get_mut(&participant_id) {
                if !participant.institutions.iter().any(|i| i.uuid == institution_id) {
                    participant.institutions.push(ParticipantInstitution {
                        uuid: institution_id,
                        clash_severity: 100
                    });
                }
            }
        }

        for (participant_id, institution_id) in self.deleted_institution_clashes {
            if let Some(participant) = relevant_participants.get_mut(&participant_id) {
                participant.institutions.retain(|i| i.uuid != institution_id);
            }
        }

        for participant in relevant_participants.values() {
            g.add(
                Entity::Participant(participant.clone())
            );
        }

        Ok(
            g
        )
        /*
        let all_clashes = ParticipantClash::get_many(db, self.updated_clashes.iter().map(|c| c.clash_id).collect()).await?;
        let mut all_clashes_by_id = all_clashes.into_iter().map(|c| (c.uuid, c)).collect::<std::collections::HashMap<_, _>>();
        let mut g = EntityGroup::new(self.tournament_id);

        for update in self.updated_clashes {
            let clash = all_clashes_by_id.remove(&update.clash_id);
            //We skip clashes that are not found. This allows us to more gracefully deal with
            //duplicated clash ids in the update.
            if let Some(mut clash) = clash {
                clash.is_approved = update.approve;
                clash.was_seen = true;
                g.add(
                    Entity::ParticipantClash(clash)
                );    
            }
        }

        for clash_id in self.deleted_clashes {
            g.delete(
                EntityTypeId::ParticipantClash,
                clash_id
            );
        }
        Ok(
            g
        )        */
    }
}