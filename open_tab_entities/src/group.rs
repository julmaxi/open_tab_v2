use std::{collections::{HashMap, HashSet}, fmt::Debug, hash::Hash};

use itertools::{Itertools, izip};
use open_tab_macros::EntityCollection;
use serde::{Serialize, Deserialize};
use sea_orm::{prelude::*, ActiveValue, IntoActiveModel, QueryOrder, QuerySelect};

use crate::{domain::{ballot::Ballot, ballot_speech_timing::BallotSpeechTiming, debate::TournamentDebate, debate_backup_ballot::DebateBackupBallot, entity::{BatchBoundTournamentEntityTrait, LoadEntity}, feedback_form::FeedbackForm, feedback_question::FeedbackQuestion, feedback_response::FeedbackResponse, participant::Participant, participant_clash::ParticipantClash, round::TournamentRound, team::Team, tournament::Tournament, tournament_break::TournamentBreak, tournament_institution::TournamentInstitution, tournament_plan_edge::TournamentPlanEdge, tournament_plan_node::TournamentPlanNode, tournament_venue::TournamentVenue, BoundTournamentEntityTrait}, schema::tournament_log};


#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum EntityState<E, T> {
    Exists(E),
    Deleted{uuid: Uuid, type_: T}
}

impl<E, T> EntityState<E, T> where T: EntityTypeIdTrait, E: EntityGroupEntityTrait<T> {
    pub fn get_uuid(&self) -> Uuid {
        match self {
            EntityState::Exists(e) => e.get_uuid(),
            EntityState::Deleted{uuid, ..} => *uuid,
        }
    }
    pub fn get_name(&self) -> String {
        match self {
            EntityState::Exists(e) => e.get_name(),
            EntityState::Deleted{ ..} => panic!()//name.clone(),
        }
    }
    pub fn get_type(&self) -> T {
        match self {
            EntityState::Exists(e) => e.get_type(),
            EntityState::Deleted{type_, ..} => type_.clone()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct VersionedEntity<E, T> {
    pub version: Uuid,
    pub entity: EntityState<E, T>
}


pub trait EntityGroupEntityTrait<T> : Clone {
    fn get_uuid(&self) -> Uuid;
    fn get_name(&self) -> String;
    fn get_type(&self) -> T;
    fn get_processing_order(&self) -> u64;
    fn get_related_uuids(&self) -> Vec<Uuid>;
}
 

pub trait EntityTypeIdTrait : Clone + Copy + Debug + PartialEq + Eq + Send + Sync + From<String> + Hash + PartialOrd + Ord {
    fn as_str(&self) -> &'static str;
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, EntityCollection)]
pub enum Entity {
    Tournament(Tournament),
    TournamentInstitution(TournamentInstitution),
    Team(Team),
    TournamentRound(TournamentRound),
    Participant(Participant),
    ParticipantClash(ParticipantClash),
    TournamentVenue(TournamentVenue),
    Ballot(Ballot),
    TournamentDebate(TournamentDebate),
    DebateBackupBallot(DebateBackupBallot),
    TournamentBreak(TournamentBreak),
    FeedbackQuestion(FeedbackQuestion),
    FeedbackForm(FeedbackForm),
    FeedbackResponse(FeedbackResponse),
    TournamentPlanNode(TournamentPlanNode),
    TournamentPlanEdge(TournamentPlanEdge),
    BallotSpeechTiming(BallotSpeechTiming),
}

pub trait GroupedEntityMapTrait<T, E> where T: EntityTypeIdTrait, E: EntityGroupEntityTrait<T> {
    fn new() -> Self;
    fn add(&mut self, entity: E);
    fn into_groups<C>(self) -> HashMap<T, Box<dyn BatchBoundTournamentEntityTrait<C>>> where C: ConnectionTrait;
}

#[async_trait::async_trait]
pub trait EntityDeletionGroupTrait<T> {
    fn new() -> Self;
    fn add(&mut self, entity_type: T, uuid: Uuid);
    async fn execute<C>(&self, db: &C) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait;
}

struct TestGroup {
    ballots: Vec<Ballot>,
    participants: Vec<Participant>,
}

impl GroupedEntityMapTrait<EntityTypeId, Entity> for TestGroup {
    fn new() -> Self {
        Self {
            ballots: Vec::new(),
            participants: Vec::new()
        }
    }
    fn add(&mut self, entity: Entity) {
        match entity {
            Entity::Ballot(b) => self.ballots.push(b),
            Entity::Participant(p) => self.participants.push(p),
            _ => {}
        }
    }

    fn into_groups<C>(self) -> HashMap<EntityTypeId, Box<dyn BatchBoundTournamentEntityTrait<C>>> where C: ConnectionTrait {
        let mut out : HashMap<EntityTypeId, Box<dyn BatchBoundTournamentEntityTrait<C>>> = HashMap::new();
        out.insert(EntityTypeId::Ballot, Box::new(self.ballots));
        out.insert(EntityTypeId::Participant, Box::new(self.participants));
        out
    }
}

#[derive(Clone)]
pub enum NewEntityState<E> {
    Exists(E),
    Deleted
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityOperationType {
    Update,
    Delete
}

pub struct EntityChangeSet<T, E, G, D> {
    pub entity_states: HashMap<(T, Uuid), NewEntityState<E>>,
    operation_log: Vec<(T, Uuid, EntityOperationType)>,
    altered_types: HashSet<T>,
    pub tournament_id: Uuid,
    _group_map_type: std::marker::PhantomData<G>,
    _delete_map_type: std::marker::PhantomData<D>
}

impl<T, E, G, D> EntityChangeSet<T, E, G, D> where E: EntityGroupEntityTrait<T>, T: EntityTypeIdTrait, G: GroupedEntityMapTrait<T, E>, D: EntityDeletionGroupTrait<T> {
    pub fn new(tournament_id: Uuid) -> Self {
        Self {
            entity_states: HashMap::new(),
            operation_log: Vec::new(),
            altered_types: HashSet::new(),
            tournament_id,
            _group_map_type: std::marker::PhantomData,
            _delete_map_type: std::marker::PhantomData
        }
    }

    pub fn new_from_entities(
        tournament_id: Uuid,
        entities: Vec<E>
    ) -> Self {
        let mut out = Self::new(tournament_id);
        for e in entities {
            out.add(e);
        }
        out
    }

    pub fn has_changes_for_type(&self, type_: T) -> bool {
        self.altered_types.contains(&type_)
    }

    pub fn has_changes_for_types(&self, types: Vec<T>) -> bool {
        types.iter().any(|t| self.has_changes_for_type(*t))
    }

    pub fn add(&mut self, entity: E) {
        let uuid = entity.get_uuid();
        let type_ = entity.get_type();
        self.entity_states.insert((entity.get_type(), uuid), NewEntityState::Exists(entity));

        self.operation_log.push((type_, uuid, EntityOperationType::Update));

        self.altered_types.insert(type_);
    }

    pub fn delete(&mut self, entity_type: T, uuid: Uuid) {
        self.entity_states.insert((entity_type, uuid), NewEntityState::Deleted);
        self.operation_log.push((entity_type, uuid, EntityOperationType::Delete));

        self.altered_types.insert(entity_type);
    }

    pub async fn save_all_and_log<C>(&self, db: &C) -> Result<Uuid, anyhow::Error> where C: sea_orm::ConnectionTrait {
        self.save_all_with_options_and_log(db, false).await
    }

    pub async fn save_all_with_options_and_log<C>(&self, db: &C, guarantee_insert: bool) -> Result<Uuid, anyhow::Error> where C: sea_orm::ConnectionTrait {
        self.save_all_with_options(db, guarantee_insert).await?;
        let head = self.save_log(db).await?;

        Ok(head)
    }

    pub async fn save_all<C>(&self, db: &C) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        self.save_all_with_options(db, false).await
    }

    pub fn as_group_map(&self) -> G {
        let mut updated_group = G::new();
        let mut entity_states = self.entity_states.clone();

        for (type_, uuid, op) in self.operation_log.iter() {
            match op {
                EntityOperationType::Update => {
                    let entity = entity_states.remove(&(type_.clone(), *uuid)).unwrap();
                    if let NewEntityState::Exists(e) = entity {
                        updated_group.add(e);
                    }
                },
                EntityOperationType::Delete => {}
            }
        }

        updated_group
    }

    pub async fn save_all_with_options<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut updated_group = G::new();
        let mut deleted_group = D::new();

        //Ensure we only process each entity once
        let operation_order = self.operation_log.clone().into_iter().rev().unique_by(|(t, u, _)| (t.as_str().to_string(), u.clone())).rev().collect_vec();

        let mut entity_states = self.entity_states.clone();

        for (type_, uuid, op) in operation_order.into_iter() {
            match op {
                EntityOperationType::Update => {
                    let entity = entity_states.remove(&(type_, uuid)).unwrap();
                    if let NewEntityState::Exists(e) = entity {
                        updated_group.add(e);
                    }
                },
                EntityOperationType::Delete => {
                    deleted_group.add(type_, uuid);
                }
            }
        }

        let groups = updated_group.into_groups();

        for (_, group) in groups.into_iter().sorted_by(|(a, _), (b, _)| a.cmp(b)) {
            group.save_many(db, false).await?;
        }

        deleted_group.execute(db).await?;

        Ok(())
    }

    pub async fn save_log<C>(&self, transaction: &C) -> Result<Uuid, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let last_log_entry = crate::schema::tournament_log::Entity::find()
        .filter(crate::schema::tournament_log::Column::TournamentId.eq(self.tournament_id))
        .order_by_desc(crate::schema::tournament_log::Column::SequenceIdx)
        .limit(1)
        .one(transaction)
        .await?;

        let last_sequence_idx = match &last_log_entry {
            Some(entry) => entry.sequence_idx,
            None => 0,
        };
        let mut log_head = match &last_log_entry {
            Some(entry) => entry.uuid,
            None => Uuid::nil(),
        };

        let now = chrono::offset::Local::now().naive_local();

        let new_entries = self.entity_states.iter().map(|e| e.clone()).enumerate().map(|(idx, ((type_id, uuid), _state))| {
            let version_uuid = Uuid::new_v4();
            crate::schema::tournament_log::ActiveModel {
                uuid: ActiveValue::Set(version_uuid),
                timestamp: ActiveValue::Set(now),
                sequence_idx: ActiveValue::Set(last_sequence_idx + 1 + idx as i32),
                tournament_id: ActiveValue::Set(self.tournament_id),
                target_type: ActiveValue::Set(type_id.as_str().to_string()),
                target_uuid: ActiveValue::Set(*uuid)
            }
        }).collect_vec();

        if new_entries.len() > 0 {
            log_head = new_entries[new_entries.len() - 1].uuid.clone().unwrap();
            crate::schema::tournament_log::Entity::insert_many(new_entries).exec(transaction).await?;
        }

        let mut existing_entities = crate::schema::tournament_entity::Entity::find().filter(
            crate::schema::tournament_entity::Column::Uuid.is_in(self.entity_states.keys().map(|(_, uuid)| *uuid).collect_vec())
        ).all(transaction).await?.into_iter().map(|e| (e.uuid, e.into_active_model())).collect::<HashMap<Uuid, crate::schema::tournament_entity::ActiveModel>>();

        let mut new_entities = Vec::new();

        let mut seen_uuids = HashSet::new();
        for ((t, uuid), state) in self.entity_states.iter() {
            let mut entity = if let Some(e) = existing_entities.remove(&uuid) {
                e
            } else {
                let new_entity = crate::schema::tournament_entity::ActiveModel {
                    uuid: ActiveValue::Set(*uuid),
                    tournament_id: ActiveValue::Set(self.tournament_id),
                    entity_type: ActiveValue::Set(t.as_str().to_string()),
                    is_deleted: ActiveValue::Set(false)
                };
                if seen_uuids.contains(&uuid) {
                    dbg!(t);
                    continue;
                }
                seen_uuids.insert(uuid);
                new_entities.push(new_entity);
                continue;
            };

            match state {
                NewEntityState::Exists(_) => {}
                NewEntityState::Deleted => {
                    entity.is_deleted = ActiveValue::Set(true);
                },
            }
        }

        if new_entities.len() > 0 {
            crate::schema::tournament_entity::Entity::insert_many(new_entities).exec(transaction).await?;
        }

        for (_, entity) in existing_entities.into_iter() {
            entity.save(transaction).await?;
        }

        Ok(log_head)
    }

    pub fn get_all_related_uuids(&self) -> HashSet<Uuid> {
        let mut out = HashSet::new();
        for ((t, u), e) in self.entity_states.iter() {
            out.insert(*u);
            match e {
                NewEntityState::Exists(e) => {
                    out.insert(*u);
                    out.extend(e.get_related_uuids());
                },
                NewEntityState::Deleted => {
                    out.insert(*u);
                }
            }
        }

        out
    }
}

/*
impl<T, E, G, D> From<Vec<E>> for EntityChangeSet<T, E, G, D> where E: EntityGroupEntityTrait<T>, T: EntityTypeIdTrait, G: GroupedEntityMapTrait<T, E>, D: EntityDeletionGroupTrait<T>  {
    fn from(entities: Vec<E>) -> Self {
        let mut out = Self::new();
        for e in entities {
            out.add(e);
        }
        out
    }
}
 */

pub type EntityGroup = EntityChangeSet<EntityTypeId, Entity, GroupedEntityMap, EntityDeletionGroup>;


pub async fn get_changed_entities_from_log<C>(transaction: &C, log_entries: Vec<crate::schema::tournament_log::Model>) -> Result<Vec<VersionedEntity<Entity, EntityTypeId>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
    let mut to_query : HashMap<EntityTypeId, Vec<(Uuid, Uuid)>> = HashMap::new();
    let mut original_indices: HashMap<(EntityTypeId, Uuid), usize> = HashMap::new();
    log_entries.into_iter().enumerate().for_each(|(idx, e)| {
        let type_ = EntityTypeId::from(e.target_type);
        match to_query.get_mut(&type_) {
            Some(v) => {
                v.push((e.target_uuid, e.uuid));
            },
            None => {
                to_query.insert(type_.clone(), vec![(e.target_uuid, e.uuid)]);
            }
        }
        original_indices.insert((type_.clone(), e.target_uuid), idx);
    });

    let mut all_new_entities = Vec::new();

    for (type_, found_entities) in to_query.into_iter() {
        let uuids = found_entities.iter().map(|e| e.0).collect_vec();
        let versions = found_entities.iter().map(|e| e.1).collect_vec();
        let new_entities = Entity::try_get_many_with_type(transaction, EntityTypeId::from(type_.clone()), uuids.clone()).await?;
        all_new_entities.extend(izip!(new_entities.into_iter(), versions.into_iter(), uuids).map(
            |(entity, version, uuid)| VersionedEntity {
                entity: entity.map(|e| EntityState::Exists(e)).unwrap_or(EntityState::Deleted{uuid, type_: EntityTypeId::from(type_.clone())}),
                version
            }
        ));
    };
    Ok(all_new_entities.into_iter().sorted_by_key(|e| original_indices.get(&(e.entity.get_type(), e.entity.get_uuid()))).collect())
}
