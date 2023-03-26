use std::{fmt::Display, error::Error, collections::HashMap, marker::PhantomData};

use async_trait::async_trait;
use itertools::{izip, Itertools};
use sea_orm::{prelude::*, Database, IntoActiveModel, ActiveValue};
use serde::{Serialize, Deserialize};
use sea_query::ValueTuple;


use crate::{schema::{self, adjudicator, speaker}, utilities::{BatchLoad, BatchLoadError}};

use super::TournamentEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Participant {
    pub uuid: Uuid,
    pub name: String,
    pub role: ParticipantRole,
    pub tournament_id: Uuid
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

}

#[derive(Debug, PartialEq, Eq)]
pub enum ParticipantParseError {
    DbErr(DbErr),
    MultipleRoles,
    ParticipantDoesNotExist
}

impl Display for ParticipantParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))?;
        Ok(())
    }
}

impl Error for ParticipantParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParticipantParseError::DbErr(e) => Some(e),
            _ => None
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl From<DbErr> for ParticipantParseError {
    fn from(value: DbErr) -> Self {
        ParticipantParseError::DbErr(value)
    }
}


impl Participant {
    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Participant>, ParticipantParseError> where C: ConnectionTrait {
        //let participants = schema::participant::Entity::find().filter(schema::participant::Column::Uuid.is_in(uuids.clone())).all(db).await?;
        let participants = schema::participant::Entity::batch_load_all(db, uuids).await.map_err(
            |e| match e {
                BatchLoadError::DbErr(e) => ParticipantParseError::DbErr(e),
                BatchLoadError::RowNotFound => ParticipantParseError::ParticipantDoesNotExist
            }
        )?;

        Self::load_participants(db, participants).await
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<Vec<Participant>, ParticipantParseError> where C: ConnectionTrait {
        let participants = schema::participant::Entity::find().filter(schema::participant::Column::TournamentId.eq(Some(tournament_uuid))).all(db).await?;
        Self::load_participants(db, participants).await
    }

    async fn load_participants<C>(db: &C, participants: Vec<schema::participant::Model>)  -> Result<Vec<Participant>, ParticipantParseError> where C: ConnectionTrait {
        let adjudicators = participants.load_one(schema::adjudicator::Entity, db).await?;
        let speakers = participants.load_one(schema::speaker::Entity, db).await?;
        let out : Result<Vec<Participant>, ParticipantParseError> = izip!(participants.into_iter(), speakers.into_iter(), adjudicators.into_iter())
        .map(|(part, speaker, adj)| {
            Self::from_rows(part, speaker, adj)
        })
        .collect();
        out
    }

    fn from_rows(
        participant: schema::participant::Model,
        speaker_info: Option<schema::speaker::Model>,
        adjudicator_info: Option<schema::adjudicator::Model>,
    ) -> Result<Self, ParticipantParseError> {
        let role = match (speaker_info, adjudicator_info) {
            (None, None) => panic!("Database constraint violated. Participant has neither adjudicator nor speaker info"),
            (None, Some(_adj)) => Ok(ParticipantRole::Adjudicator(Adjudicator{})),
            (Some(speaker), None) => Ok(ParticipantRole::Speaker(Speaker{team_id: speaker.team_id})),
            (Some(_), Some(_)) => Err(ParticipantParseError::MultipleRoles),
        }?;

        Ok(Participant { uuid: participant.uuid, name: participant.name, role: role, tournament_id: participant.tournament_id })
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

//        E::insert_many(self.insert.clone()).exec(db).await?;
        
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
impl TournamentEntity for Participant {
    async fn save_many<C>(db: &C, guarantee_insert: bool, entities: &Vec<&Self>) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let existing = if guarantee_insert {
            (vec![], vec![], vec![])
        }
        else {
            let participants = schema::participant::Entity::find()
            .filter(schema::participant::Column::Uuid.is_in(
                entities.iter().map(|x| x.uuid.clone()))).all(db).await?;
            
            let adjs = participants.load_one(adjudicator::Entity, db).await?;
            let speakers = participants.load_one(speaker::Entity, db).await?;

            (participants, adjs, speakers)
        };

        let existing : HashMap<Uuid, _, std::collections::hash_map::RandomState> = HashMap::from_iter(izip!(existing.0, existing.1, existing.2).into_iter().map(|e| (e.0.uuid.clone(), e)));

        let mut participant_changes = ChangeSet::new(schema::participant::Column::Uuid);
        let mut speaker_changes = ChangeSet::new(schema::speaker::Column::Uuid);
        let mut adj_changes = ChangeSet::new(schema::adjudicator::Column::Uuid);

        for ent in entities {
            let mut participant_change = schema::participant::ActiveModel {
                uuid: ActiveValue::Unchanged(ent.uuid),
                tournament_id: ActiveValue::Set(ent.tournament_id),
                name: ActiveValue::Set(ent.name.clone())
            };

            if let Some((_part_model, adj_model, speaker_model)) = existing.get(&ent.uuid) {
                participant_changes.update.push(
                    participant_change
                );
                match (&ent.role, adj_model, speaker_model) {
                    (_, None, None) => panic!("Participant has no role"),
                    (_, Some(_), Some(_)) => panic!("Participant has two roles"),
                    (ParticipantRole::Adjudicator(_adj), None, Some(speaker_model)) => {
                        speaker_changes.delete.push(
                            speaker_model.clone().into()
                        );
                        adj_changes.insert.push(
                            adjudicator::ActiveModel { uuid: ActiveValue::Set(ent.uuid) }
                        );
                    },
                    (ParticipantRole::Adjudicator(_adj), Some(m), None) => {
                        adj_changes.update.push(
                            m.clone().into()
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
                    ParticipantRole::Adjudicator(_adj) => {
                        adj_changes.insert.push(schema::adjudicator::ActiveModel {
                            uuid: ActiveValue::Set(ent.uuid),
                        });
                    },
                }
            }
        }

        participant_changes.exec(db).await?;
        speaker_changes.exec(db).await?;
        adj_changes.exec(db).await?;

        Ok(())
    }

    async fn get_tournament<C>(&self, db: &C) -> Result<Option<Uuid>, Box<dyn Error>> where C: ConnectionTrait {
        Ok(Some(self.tournament_id))
    }

}


#[test]
fn test_get_speaker() -> Result<(), ParticipantParseError> {
    let participant = Participant::from_rows(
        schema::participant::Model {
            uuid: Uuid::from_u128(400),
            tournament_id: Uuid::from_u128(100),
            name: "Test".into(),
        },
        Some(schema::speaker::Model {
            uuid: Uuid::from_u128(400),
            team_id: Some(Uuid::from_u128(200)),
        }),
        None
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
        },
        None,
        Some(schema::adjudicator::Model { uuid: Uuid::from_u128(400) })
    )?;

    assert_eq!(participant.uuid, Uuid::from_u128(400));

    if let ParticipantRole::Adjudicator(_a) = participant.role {
    }
    else {
        panic!("Participant should be Adjudicator")
    }

    Ok(())
}

#[test]
fn test_mixed_role_error() -> Result<(), ParticipantParseError> {
    let participant = Participant::from_rows(
        schema::participant::Model {
            uuid: Uuid::from_u128(400),
            tournament_id: Uuid::from_u128(100),
            name: "Test".into(),
        },
        Some(schema::speaker::Model {
            uuid: Uuid::from_u128(400),
            team_id: Some(Uuid::from_u128(200)),
        }),
        Some(schema::adjudicator::Model { uuid: Uuid::from_u128(400) })
    );

    assert_eq!(participant, Err(ParticipantParseError::MultipleRoles));

    Ok(())
}
