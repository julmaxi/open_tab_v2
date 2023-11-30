use std::{collections::HashMap, fmt::Debug, hash::Hash};

use itertools::{Itertools, izip};
use open_tab_macros::EntityGroup;
use serde::{Serialize, Deserialize};
use sea_orm::{prelude::*, QueryOrder, QuerySelect, ActiveValue};

use crate::{domain::{participant::Participant, ballot::Ballot, tournament::Tournament, debate::TournamentDebate, round::TournamentRound, team::Team, tournament_institution::TournamentInstitution, participant_clash::ParticipantClash, debate_backup_ballot::DebateBackupBallot, tournament_break::TournamentBreak, entity::LoadEntity, tournament_venue::TournamentVenue, feedback_question::FeedbackQuestion, feedback_form::FeedbackForm, feedback_response::FeedbackResponse, tournament_plan_node::TournamentPlanNode, tournament_plan_edge::TournamentPlanEdge}, schema::tournament_log};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum EntityState<E, T> {
    Exists(E),
    Deleted{uuid: Uuid, type_: T}
}

impl<E> EntityState<E, <<E as EntityGroupEntityTrait>::EntityGroup as EntityGroupTrait>::TypeId> where E: EntityGroupEntityTrait {
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
    pub fn get_type(&self) -> <E::EntityGroup as EntityGroupTrait>::TypeId {
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

pub trait EntityGroupEntityTrait {
    type EntityGroup: EntityGroupTrait;

    fn get_uuid(&self) -> Uuid;
    fn get_name(&self) -> String;
    fn get_type(&self) -> <Self::EntityGroup as EntityGroupTrait>::TypeId;
    fn get_processing_order(&self) -> u64;
}

pub trait EntityTypeId : Clone + Copy + Debug + PartialEq + Eq + Send + Sync + From<String> + Hash {
    fn as_str(&self) -> &'static str;
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, EntityGroup)]
pub enum Entity {
    Tournament(Tournament),
    TournamentInstitution(TournamentInstitution),
    Team(Team),
    Participant(Participant),
    ParticipantClash(ParticipantClash),
    TournamentRound(TournamentRound),
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
}




#[async_trait::async_trait]
pub trait EntityGroupTrait {
    type TypeId: Debug + Clone + Serialize + Deserialize<'static> + PartialEq + Eq + Send + Sync;

    fn new() -> Self;
    fn add(&mut self, e: Entity);
    fn delete(&mut self, type_: Self::TypeId, uuid: Uuid);
    fn add_versioned(&mut self, e: Entity, version: Uuid);
    fn delete_versioned(&mut self, type_: Self::TypeId, uuid: Uuid, version: Uuid);
    fn get_entity_ids(&self) -> Vec<(Self::TypeId, Uuid)>;
    async fn get_all_deletion_tournaments<C>(&self, db: &C) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait;
    async fn get_all_tournaments<C>(&self, db: &C) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait;
    async fn save_all_with_options<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait;
    async fn save_log_with_tournament_id<C>(&self, transaction: &C, tournament_id: Uuid) -> Result<Uuid, anyhow::Error> where C: sea_orm::ConnectionTrait;
    async fn get_many_with_type<C>(transaction: &C, type_name: Self::TypeId, uuids: Vec<Uuid>) -> Result<Vec<Entity>, anyhow::Error> where C: sea_orm::ConnectionTrait;
    async fn try_get_many_with_type<C>(transaction: &C, type_name: Self::TypeId, uuids: Vec<Uuid>) -> Result<Vec<Option<Entity>>, anyhow::Error> where C: sea_orm::ConnectionTrait;

    async fn save_all<C>(&self, db: &C) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        self.save_all_with_options(db, false).await
    }

    async fn save_all_and_log_for_tournament<C>(&self, db: &C, tournament_id: Uuid) -> Result<Uuid, anyhow::Error> where C: sea_orm::ConnectionTrait {
        self.save_all_with_options_and_log_for_tournament(db, false, tournament_id).await
    }

    async fn save_all_with_options_and_log_for_tournament<C>(&self, db: &C, guarantee_insert: bool, tournament_id: Uuid) -> Result<Uuid, anyhow::Error> where C: sea_orm::ConnectionTrait {
        self.save_all_with_options(db, guarantee_insert).await?;
        self.save_log_with_tournament_id(db, tournament_id).await
    }

    fn new_with_entities(entities: Vec<Entity>) -> Self where Self: Sized {
        let mut group = Self::new();
        entities.into_iter().for_each(|e| group.add(e));
        group
    }
}


pub async fn get_changed_entities_from_log<C>(transaction: &C, log_entries: Vec<crate::schema::tournament_log::Model>) -> Result<Vec<VersionedEntity<Entity, EntityType>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
    let mut to_query : HashMap<EntityType, Vec<(Uuid, Uuid)>> = HashMap::new();
    let mut original_indices: HashMap<(EntityType, Uuid), usize> = HashMap::new();
    log_entries.into_iter().enumerate().for_each(|(idx, e)| {
        let type_ = EntityType::from(e.target_type);
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
        let new_entities = EntityGroup::try_get_many_with_type(transaction, EntityType::from(type_.clone()), uuids.clone()).await?;
        all_new_entities.extend(izip!(new_entities.into_iter(), versions.into_iter(), uuids).map(
            |(entity, version, uuid)| VersionedEntity {
                entity: entity.map(|e| EntityState::Exists(e)).unwrap_or(EntityState::Deleted{uuid, type_: EntityType::from(type_.clone())}),
                version
            }
        ));
    };
    Ok(all_new_entities.into_iter().sorted_by_key(|e| original_indices.get(&(e.entity.get_type(), e.entity.get_uuid()))).collect())
}
