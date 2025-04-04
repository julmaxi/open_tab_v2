use std::{collections::{HashMap, hash_map::RandomState}, vec};

use async_trait::async_trait;
use base64::Engine;
use itertools::{izip, Itertools};
use sea_orm::{prelude::*, IntoActiveModel, ActiveValue};
use serde::{Serialize, Deserialize};
use sea_query::ValueTuple;


use crate::{schema::{self, adjudicator, speaker, participant_tournament_institution, adjudicator_availability_override}, utilities::BatchLoad};

use super::{entity::{LoadEntity, TournamentEntityTrait}, tournament::Tournament, BoundTournamentEntityTrait};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Participant {
    pub uuid: Uuid,
    pub name: String,
    pub role: ParticipantRole,
    pub tournament_id: Uuid,
    pub institutions: Vec<ParticipantInstitution>,
    pub registration_key: Option<Vec<u8>>,
    pub is_anonymous: bool,
    pub break_category_id: Option<Uuid>,
}

impl Participant {
    pub fn new_with_uuid(
        uuid: Uuid,
        name: String,
        role: ParticipantRole,
        tournament_id: Uuid,
    ) -> Self {
        Self {
            uuid,
            name,
            role,
            tournament_id,
            institutions: vec![],
            registration_key: None,
            is_anonymous: false,
            break_category_id: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct ParticipantInstitution {
    pub uuid: Uuid,
    pub clash_severity: u16
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum ParticipantRole {
    Speaker(Speaker),
    Adjudicator(Adjudicator),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Speaker {
    pub team_id: Option<Uuid>
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default)]
pub struct Adjudicator {
    pub chair_skill: i16,
    pub panel_skill: i16,
    pub unavailable_rounds: Vec<Uuid>
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ParticipantParseError {
    #[error("Database error: {0}")]
    DbErr(#[from] DbErr),
    #[error("Multiple roles for participant")]
    MultipleRoles,
    #[error("Participant does not exist")]
    ParticipantDoesNotExist
}

#[async_trait]
impl LoadEntity for Participant {
    async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Participant>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let participants = schema::participant::Entity::batch_load(db, uuids).await?;

        let has_value = participants.iter().map(|b| b.is_some()).collect_vec();

        let mut participants = Self::load_participants(db, participants.into_iter().filter_map(|val| val).collect()).await?.into_iter();

        let out = has_value.into_iter().map(|has_v| {
            if has_v {
                participants.next()
            }
            else {
                None
            }
        });
        Ok(out.collect())
    }
}


impl Participant {
    pub fn encode_registration_key(uuid: Uuid, key: &[u8]) -> String {
        let mut registration_secret = [0; 48];
        registration_secret[0..16].copy_from_slice(uuid.as_bytes());
        registration_secret[16..48].copy_from_slice(key);

        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&registration_secret)
    }

    pub fn decode_registration_key(key: String) -> Result<(Uuid, Vec<u8>), anyhow::Error> {
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&key)?;
        let uuid = Uuid::from_slice(&decoded[0..16])?;
        let key = decoded[16..48].to_vec();
        Ok((uuid, key))
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<Vec<Participant>, ParticipantParseError> where C: sea_orm::ConnectionTrait {
        let participants = schema::participant::Entity::find().filter(schema::participant::Column::TournamentId.eq(Some(tournament_uuid))).all(db).await?;
        Self::load_participants(db, participants).await
    }

    pub async fn get_all_adjudicators_in_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<Vec<Participant>, ParticipantParseError> where C: sea_orm::ConnectionTrait {
        let participants = schema::participant::Entity::find().filter(
            schema::participant::Column::TournamentId.eq(Some(tournament_uuid))
        ).inner_join(
            schema::adjudicator::Entity
        ).all(db).await?;
        Self::load_participants(db, participants).await
    }

    async fn load_participants<C>(db: &C, participants: Vec<schema::participant::Model>)  -> Result<Vec<Participant>, ParticipantParseError> where C: sea_orm::ConnectionTrait {
        let adjudicators = participants.load_one(schema::adjudicator::Entity, db).await?;

        let adjudicator_overides = adjudicator_availability_override::Entity::find().filter(
            adjudicator_availability_override::Column::AdjudicatorId.is_in(adjudicators.iter().filter_map(|adj| adj.as_ref().map(|adj| adj.uuid)).collect_vec())
        ).all(db).await?.into_iter().map(
            |m| (m.adjudicator_id, m.round_id)
        ).into_group_map();

        let speakers = participants.load_one(schema::speaker::Entity, db).await?;
        let institutions = participants.load_many(participant_tournament_institution::Entity, db).await?;
        
        let out : Result<Vec<Participant>, ParticipantParseError> = izip!(participants.into_iter(), speakers.into_iter(), adjudicators.into_iter(), institutions.into_iter())
        .map(|(part, speaker, adj, inst)| {
            Self::from_rows(part, speaker, adj, inst, &adjudicator_overides)
        })
        .collect();
        out
    }

    fn from_rows(
        participant: schema::participant::Model,
        speaker_info: Option<schema::speaker::Model>,
        adjudicator_info: Option<schema::adjudicator::Model>,
        institution_info: Vec<participant_tournament_institution::Model>,
        adjudicator_overides: &HashMap<Uuid, Vec<Uuid>>
    ) -> Result<Self, ParticipantParseError> {
        let role = match (speaker_info, adjudicator_info) {
            (None, None) => panic!("Database constraint violated. Participant has neither adjudicator nor speaker info"),
            (None, Some(adj)) => Ok(ParticipantRole::Adjudicator(Adjudicator{
                chair_skill: adj.chair_skill,
                panel_skill: adj.panel_skill,
                unavailable_rounds: adjudicator_overides.get(&adj.uuid).cloned().unwrap_or(vec![])
            })),
            (Some(speaker), None) => Ok(ParticipantRole::Speaker(Speaker{team_id: speaker.team_id})),
            (Some(_), Some(_)) => Err(ParticipantParseError::MultipleRoles),
        }?;

        let institutions = institution_info.into_iter().map(
            |institution| ParticipantInstitution {
                uuid: institution.institution_id,
                clash_severity: institution.clash_severity as u16
            }
        ).collect();

        Ok(Participant {
            uuid: participant.uuid,
            name: participant.name,
            registration_key: participant.registration_key,
            role: role,
            tournament_id: participant.tournament_id,
            institutions: institutions,
            is_anonymous: participant.is_anonymous,
            break_category_id: participant.break_category_id
        })
    }
}

#[derive(Debug)]
struct ChangeSet<A, C> {
    update: Vec<A>,
    delete: Vec<A>,
    insert: Vec<A>,
    primary_key_col: C
}

use sea_orm::sea_query::FromValueTuple;

impl<A, C, E, P> ChangeSet<A, C> where A: ActiveModelTrait<Entity = E> + IntoActiveModel<A>, E: EntityTrait<Column=C, PrimaryKey = P>, P: PrimaryKeyTrait<ValueType = Uuid>, C: ColumnTrait, <E as sea_orm::EntityTrait>::Model: IntoActiveModel<A> {
    fn new(primary_key_col: C) -> ChangeSet<A, C> {
        ChangeSet {
            update: vec![],
            delete: vec![],
            insert: vec![],
            primary_key_col
        }
    }
    
    async fn exec<Conn>(&self, db: &Conn) -> Result<(), DbErr> where Conn: ConnectionTrait {
        let e : Vec<ValueTuple> = self.delete.iter().map(|e| e.get_primary_key_value().unwrap()).collect_vec();
        let c : Vec<Uuid> = e.into_iter().map(|x| Uuid::from_value_tuple(x)).collect_vec();
        E::delete_many().filter(
            self.primary_key_col.is_in(c)
        ).exec(db).await?;
        
        for e in self.insert.clone().into_iter() {
            E::insert(e).exec(db).await?;
        }

        for m in self.update.iter() {
            let m : A = m.clone();
            E::update(m).exec(db).await?;
        }

        Ok(())
    }
}


#[async_trait]
impl<C> BoundTournamentEntityTrait<C> for Participant where C: sea_orm::ConnectionTrait {
    async fn save_many(db: &C, guarantee_insert: bool, entities: &Vec<&Self>) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        let (existing, adjudicator_overrides) = if guarantee_insert {
            ((vec![], vec![], vec![], vec![]), HashMap::new())
        }
        else {
            let participants = schema::participant::Entity::find()
            .filter(schema::participant::Column::Uuid.is_in(
                entities.iter().map(|x| x.uuid.clone()))).all(db).await?;
            
            let adjs = participants.load_one(adjudicator::Entity, db).await?;
            let speakers = participants.load_one(speaker::Entity, db).await?;
            let institutions = participants.load_many(participant_tournament_institution::Entity, db).await?;

            let adjudicator_overrides = adjudicator_availability_override::Entity::find().filter(
                adjudicator_availability_override::Column::AdjudicatorId.is_in(adjs.iter().filter_map(|adj| adj.as_ref().map(|adj| adj.uuid)).collect_vec())
            ).all(db).await?.into_iter().map(
                |m| (m.adjudicator_id, m.round_id)
            ).into_group_map();

            ((participants, adjs, speakers, institutions), adjudicator_overrides)
        };

        let existing : HashMap<Uuid, _, std::collections::hash_map::RandomState> = HashMap::from_iter(izip!(existing.0, existing.1, existing.2, existing.3).into_iter().map(|e| (e.0.uuid.clone(), e)));

        let mut participant_changes = ChangeSet::new(schema::participant::Column::Uuid);
        let mut speaker_changes = ChangeSet::new(schema::speaker::Column::Uuid);
        let mut adj_changes = ChangeSet::new(schema::adjudicator::Column::Uuid);
        // Institutions have a composite primary key, so we can't use ChangeSet. Same for overrides.
        let mut institution_insertions = vec![];
        let mut institution_updates = vec![];
        let mut institution_deletes = vec![];

        let mut override_insertions = vec![];
        let mut override_deletions = vec![];

        for ent in entities {
            let mut participant_change = schema::participant::ActiveModel {
                uuid: ActiveValue::Unchanged(ent.uuid),
                tournament_id: ActiveValue::Set(ent.tournament_id),
                name: ActiveValue::Set(ent.name.clone()),
                registration_key: ActiveValue::Set(ent.registration_key.clone()),
                is_anonymous: ActiveValue::Set(ent.is_anonymous),
                break_category_id: ActiveValue::Set(ent.break_category_id),
            };

            if let Some((_part_model, adj_model, speaker_model, institution_models)) = existing.get(&ent.uuid) {
                participant_changes.update.push(
                    participant_change
                );
                match (&ent.role, adj_model, speaker_model) {
                    (_, None, None) => panic!("Participant has no role"),
                    (_, Some(_), Some(_)) => panic!("Participant has two roles"),
                    (ParticipantRole::Adjudicator(adj), None, Some(speaker_model)) => {
                        speaker_changes.delete.push(
                            speaker_model.clone().into()
                        );
                        adj_changes.insert.push(
                            adjudicator::ActiveModel { uuid: ActiveValue::Set(ent.uuid), chair_skill: ActiveValue::Set(adj.chair_skill), panel_skill: ActiveValue::Set(adj.panel_skill) }
                        );
                    },
                    (ParticipantRole::Adjudicator(adj), Some(_m), None) => {
                        adj_changes.update.push(
                            adjudicator::ActiveModel { uuid: ActiveValue::Set(ent.uuid), chair_skill: ActiveValue::Set(adj.chair_skill), panel_skill: ActiveValue::Set(adj.panel_skill) }
                        )
                    },
                    (ParticipantRole::Speaker(speaker), None, Some(speaker_model)) => {
                        let mut active : speaker::ActiveModel = speaker_model.clone().into();
                        active.team_id = ActiveValue::Set(speaker.team_id);
                        speaker_changes.update.push(
                            active
                        );
                    }
                    (ParticipantRole::Speaker(speaker), Some(adj), None)  => {
                        adj_changes.delete.push(
                            adj.clone().into()
                        );
                        speaker_changes.insert.push(
                            speaker::ActiveModel { uuid: ActiveValue::Set(ent.uuid), team_id: ActiveValue::Set(speaker.team_id) }
                        );
                    },
                };

                let mut existing_institutions : HashMap<Uuid, &participant_tournament_institution::Model, RandomState> = HashMap::from_iter(institution_models.into_iter().map(|x| (x.institution_id, x)));
                for institution in ent.institutions.iter() {
                    let previous_inst = existing_institutions.remove(&institution.uuid);
                    let mut update = participant_tournament_institution::ActiveModel {
                        participant_id: ActiveValue::Unchanged(ent.uuid),
                        institution_id: ActiveValue::Set(institution.uuid),
                        clash_severity: ActiveValue::Set(institution.clash_severity as i16)
                    };

                    if let Some(_) = previous_inst {
                        institution_updates.push(
                            update
                        )
                    }
                    else {
                        update.participant_id = ActiveValue::Set(ent.uuid);
                        institution_insertions.push(
                            update
                        )
                    }
                }

                for inst in existing_institutions.values() {
                    institution_deletes.push(
                        (*inst).clone().into_active_model()
                    )
                }
            }
            else {
                participant_change.uuid = ActiveValue::Set(ent.uuid);
                participant_changes.insert.push(
                    participant_change
                );

                match &ent.role {
                    ParticipantRole::Speaker(speaker) => {
                        speaker_changes.insert.push(schema::speaker::ActiveModel {
                            uuid: ActiveValue::Set(ent.uuid),
                            team_id: ActiveValue::Set(speaker.team_id)
                        });
                    },
                    ParticipantRole::Adjudicator(adj) => {
                        adj_changes.insert.push(schema::adjudicator::ActiveModel {
                            uuid: ActiveValue::Set(ent.uuid),
                            chair_skill: ActiveValue::Set(adj.chair_skill),
                            panel_skill: ActiveValue::Set(adj.panel_skill),
                        });
                    },
                }

                institution_insertions.extend(ent.institutions.iter().map(
                    |institution| participant_tournament_institution::ActiveModel {
                        participant_id: ActiveValue::Unchanged(ent.uuid),
                        institution_id: ActiveValue::Set(institution.uuid),
                        clash_severity: ActiveValue::Set(institution.clash_severity as i16)
                    }
                ).collect_vec());
            }
        
            match &ent.role {
                ParticipantRole::Adjudicator(adj) => {
                    let empty = &vec![];
                    let previous_unavailable = adjudicator_overrides.get(&ent.uuid).unwrap_or(&empty);
                    let current_unavailable = &adj.unavailable_rounds;

                    let to_insert = current_unavailable.iter().filter(|x| !previous_unavailable.contains(x)).map(|x| adjudicator_availability_override::ActiveModel {
                        adjudicator_id: ActiveValue::Set(ent.uuid),
                        round_id: ActiveValue::Set(*x)
                    });
                    let to_delete = previous_unavailable.iter().filter(|x| !current_unavailable.contains(x)).map(|x| adjudicator_availability_override::ActiveModel {
                        adjudicator_id: ActiveValue::Set(ent.uuid),
                        round_id: ActiveValue::Set(*x)
                    });

                    override_insertions.extend(to_insert);
                    override_deletions.extend(to_delete);
                }
                _ => {}
            }
        }

        participant_changes.exec(db).await?;
        speaker_changes.exec(db).await?;
        adj_changes.exec(db).await?;

        for insert in institution_insertions {
            insert.insert(db).await?;
        }

        for update in institution_updates {
            update.update(db).await?;
        }

        for delete in institution_deletes {
            delete.delete(db).await?;
        }

        for insert in override_insertions {
            insert.insert(db).await?;
        }

        for delete in override_deletions {
            delete.delete(db).await?;
        }

        Ok(())
    }

    async fn get_tournament(&self, _db: &C) -> Result<Option<Uuid>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(Some(self.tournament_id))
    }

    async fn delete_many(db: &C, ids: Vec<Uuid>) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        schema::participant::Entity::delete_many().filter(schema::participant::Column::Uuid.is_in(ids)).exec(db).await?;
        Ok(())
    }
}

impl TournamentEntityTrait for Participant {
    fn get_related_uuids(&self) -> Vec<Uuid> {
        let mut out = vec![self.uuid, self.tournament_id];

        match &self.role {
            ParticipantRole::Speaker(s) => {
                if let Some(team_id) = s.team_id {
                    out.push(team_id);
                }
            },
            ParticipantRole::Adjudicator(a) => {
                out.extend(a.unavailable_rounds.clone());
            }
        }

        out.extend(self.institutions.iter().map(|x| x.uuid));

        out
    }
}


#[test]
fn test_get_speaker() -> Result<(), ParticipantParseError> {
    let participant = Participant::from_rows(
        schema::participant::Model {
            uuid: Uuid::from_u128(400),
            tournament_id: Uuid::from_u128(100),
            name: "Test".into(),
            registration_key: None,
            is_anonymous: false,
            break_category_id: None,
        },
        Some(schema::speaker::Model {
            uuid: Uuid::from_u128(400),
            team_id: Some(Uuid::from_u128(200)),
        }),
        None,
        vec![],
        &HashMap::new()
    )?;

    assert_eq!(participant.uuid, Uuid::from_u128(400));

    if let ParticipantRole::Speaker(s) = participant.role {
        assert_eq!(s.team_id, Some(Uuid::from_u128(200)));
    }
    else {
        panic!("Participant should be Speaker")
    }

    Ok(())
}

#[test]
fn test_get_adjudicator() -> Result<(), ParticipantParseError> {
    let participant = Participant::from_rows(
        schema::participant::Model {
            uuid: Uuid::from_u128(400),
            tournament_id: Uuid::from_u128(100),
            name: "Test".into(),
            registration_key: None,
            is_anonymous: false,
            break_category_id: None,
        },
        None,
        Some(schema::adjudicator::Model { uuid: Uuid::from_u128(400), chair_skill: 0, panel_skill: 0 }),
        vec![],
        &HashMap::new()
    )?;

    assert_eq!(participant.uuid, Uuid::from_u128(400));

    if let ParticipantRole::Adjudicator(_a) = participant.role {
    }
    else {
        panic!("Participant should be Adjudicator")
    }

    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_mixed_role_error() -> Result<(), ParticipantParseError> {
        let participant = Participant::from_rows(
            schema::participant::Model {
                uuid: Uuid::from_u128(400),
                tournament_id: Uuid::from_u128(100),
                name: "Test".into(),
                registration_key: None,
                is_anonymous: false,
                break_category_id: None,
            },
            Some(schema::speaker::Model {
                uuid: Uuid::from_u128(400),
                team_id: Some(Uuid::from_u128(200)),
            }),
            Some(schema::adjudicator::Model { uuid: Uuid::from_u128(400), chair_skill: 0, panel_skill: 0 }),
            vec![],
            &HashMap::new()
        );
    
        assert_eq!(participant, Err(ParticipantParseError::MultipleRoles));
    
        Ok(())
    }
    
    #[test]
    fn test_get_institutions() -> Result<(), ParticipantParseError> {
        let participant = Participant::from_rows(
            schema::participant::Model {
                uuid: Uuid::from_u128(400),
                tournament_id: Uuid::from_u128(100),
                name: "Test".into(),
                registration_key: None,
                is_anonymous: false,
                break_category_id: None,
            },
            Some(schema::speaker::Model {
                uuid: Uuid::from_u128(400),
                team_id: Some(Uuid::from_u128(200)),
            }),
            None,
            vec![
                schema::participant_tournament_institution::Model {
                    participant_id: Uuid::from_u128(400),
                    institution_id: Uuid::from_u128(500),
                    clash_severity: 200
                },
                schema::participant_tournament_institution::Model {
                    participant_id: Uuid::from_u128(400),
                    institution_id: Uuid::from_u128(501),
                    clash_severity: 1
                }
            ],
            &HashMap::new()
        )?;
    
        let mut sorted_institutions = participant.institutions.clone();
        sorted_institutions.sort_by_key(|p| p.uuid);
    
        assert_eq!(sorted_institutions, vec![
            ParticipantInstitution {
                uuid: Uuid::from_u128(500),
                clash_severity: 200
            },
            ParticipantInstitution {
                uuid: Uuid::from_u128(501),
                clash_severity: 1
            },
        ]);
    
        Ok(())
    }    
}
