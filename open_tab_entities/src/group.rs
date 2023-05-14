use std::{error::Error, f32::consts::E, collections::HashMap, hash::Hash};

use itertools::Itertools;
use open_tab_macros::EntityGroup;
use serde::{Serialize, Deserialize};
use sea_orm::{prelude::*, QueryOrder, QuerySelect, ActiveValue};

use crate::{domain::{participant::Participant, ballot::Ballot, TournamentEntity, tournament::Tournament, debate::TournamentDebate, round::TournamentRound, team::Team, tournament_institution::TournamentInstitution, participant_clash::ParticipantClash, debate_backup_ballot::DebateBackupBallot, tournament_break::TournamentBreak, entity::LoadEntity}, schema::tournament_log};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct VersionedEntity<E> {
    pub version: Uuid,
    pub entity: E
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, EntityGroup)]
pub enum Entity {
    Tournament(Tournament),
    TournamentInstitution(TournamentInstitution),
    Team(Team),
    Participant(Participant),
    ParticipantClash(ParticipantClash),
    TournamentRound(TournamentRound),
    Ballot(Ballot),
    TournamentDebate(TournamentDebate),
    DebateBackupBallot(DebateBackupBallot),
    TournamentBreak(TournamentBreak)
}

#[async_trait::async_trait]
pub trait EntityGroupTrait {
    fn new() -> Self;
    fn add(&mut self, e: Entity);
    fn add_versioned(&mut self, e: Entity, version: Uuid);
    fn get_entity_ids(&self) -> Vec<(String, Uuid)>;
    async fn get_all_tournaments<C>(&self, db: &C) -> Result<Vec<Option<Uuid>>, Box<dyn Error>> where C: ConnectionTrait;
    async fn save_all_with_options<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait;
    async fn save_log_with_tournament_id<C>(&self, transaction: &C, tournament_id: Uuid) -> Result<Uuid, Box<dyn Error>> where C: ConnectionTrait;
    async fn get_many_with_type<C>(transaction: &C, type_name: &str, uuids: Vec<Uuid>) -> Result<Vec<Entity>, Box<dyn Error>> where C: sea_orm::ConnectionTrait;

    async fn save_all<C>(&self, db: &C) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        self.save_all_with_options(db, false).await
    }

    async fn save_all_and_log_for_tournament<C>(&self, db: &C, tournament_id: Uuid) -> Result<Uuid, Box<dyn Error>> where C: ConnectionTrait {
        self.save_all_with_options_and_log_for_tournament(db, false, tournament_id).await
    }

    async fn save_all_with_options_and_log_for_tournament<C>(&self, db: &C, guarantee_insert: bool, tournament_id: Uuid) -> Result<Uuid, Box<dyn Error>> where C: ConnectionTrait {
        self.save_all_with_options(db, guarantee_insert).await?;
        self.save_log_with_tournament_id(db, tournament_id).await
    }
}

pub async fn get_changed_entities_from_log<C>(transaction: &C, log_entries: Vec<crate::schema::tournament_log::Model>) -> Result<Vec<VersionedEntity<Entity>>, Box<dyn Error>> where C: ConnectionTrait {
    let mut to_query : HashMap<String, Vec<(Uuid, Uuid)>> = HashMap::new();
    let mut original_indices: HashMap<(String, Uuid), usize> = HashMap::new();
    log_entries.into_iter().enumerate().for_each(|(idx, e)| {
        match to_query.get_mut(&e.target_type) {
            Some(v) => {
                v.push((e.target_uuid, e.uuid));
            },
            None => {
                to_query.insert(e.target_type.clone(), vec![(e.target_uuid, e.uuid)]);
            }
        }
        original_indices.insert((e.target_type, e.target_uuid), idx);
    });

    let mut all_new_entities = Vec::new();

    for (type_, found_entities) in to_query.into_iter() {
        let uuids = found_entities.iter().map(|e| e.0).collect_vec();
        let versions = found_entities.iter().map(|e| e.1).collect_vec();
        let new_entities = EntityGroup::get_many_with_type(transaction, &type_, uuids).await?;
        all_new_entities.extend(new_entities.into_iter().zip(versions.into_iter()).map(
            |(entity, version)| VersionedEntity {
                entity,
                version
            }
        ));
    };
    Ok(all_new_entities.into_iter().sorted_by_key(|e| original_indices.get(&(e.entity.get_name(), e.entity.get_uuid()))).collect())
}
