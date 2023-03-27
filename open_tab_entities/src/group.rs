use std::{error::Error, f32::consts::E, collections::HashMap};

use itertools::Itertools;
use serde::{Serialize, Deserialize};
use sea_orm::{prelude::*, QueryOrder, QuerySelect, ActiveValue};

use crate::{domain::{participant::Participant, ballot::Ballot, TournamentEntity, tournament::Tournament, debate::TournamentDebate, round::TournamentRound, team::Team, tournament_institution::TournamentInstitution, participant_clash::ParticipantClash}, schema::tournament_log};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Entity {
    Tournament(Tournament),
    TournamentInstitution(TournamentInstitution),
    Team(Team),
    Participant(Participant),
    ParticipantClash(ParticipantClash),
    TournamentRound(TournamentRound),
    Ballot(Ballot),
    TournamentDebate(TournamentDebate),
}


pub struct EntityGroups {
    pub tournaments: Vec<Tournament>,
    pub rounds: Vec<TournamentRound>,
    pub debates: Vec<TournamentDebate>,
    pub participants: Vec<Participant>,
    pub ballots: Vec<Ballot>,
    pub teams: Vec<Team>,
    pub tournament_institutions: Vec<TournamentInstitution>,
    pub participant_clashes: Vec<ParticipantClash>,
}

impl EntityGroups {
    pub fn add(&mut self, e: Entity) {
        match e {
            Entity::Participant(p) => self.participants.push(p),
            Entity::Ballot(b) => self.ballots.push(b),
            Entity::Tournament(e) => self.tournaments.push(e),
            Entity::TournamentRound(e) => self.rounds.push(e),
            Entity::TournamentDebate(e) => self.debates.push(e),
            Entity::Team(e) => self.teams.push(e),
            Entity::TournamentInstitution(e) => self.tournament_institutions.push(e),
            Entity::ParticipantClash(e) => self.participant_clashes.push(e),
        }
    }

    pub fn new() -> Self {
        EntityGroups {
            participants: vec![],
            ballots: vec![],
            tournaments: vec![],
            rounds: vec![],
            debates: vec![],
            teams: vec![],
            tournament_institutions: vec![],
            participant_clashes: vec![],
        }
    }

    pub async fn get_all_tournaments<C>(&self, db: &C) -> Result<Vec<Option<Uuid>>, Box<dyn Error>> where C: ConnectionTrait {
        let mut out = Vec::new();

        out.extend(Tournament::get_many_tournaments(db, &self.tournaments.iter().collect()).await?.into_iter());
        out.extend(Participant::get_many_tournaments(db, &self.participants.iter().collect()).await?.iter());
        out.extend(Ballot::get_many_tournaments(db, &self.ballots.iter().collect()).await?.into_iter());
        out.extend(TournamentRound::get_many_tournaments(db, &self.rounds.iter().collect()).await?.into_iter());
        out.extend(TournamentDebate::get_many_tournaments(db, &self.debates.iter().collect()).await?.into_iter());
        out.extend(Team::get_many_tournaments(db, &self.teams.iter().collect()).await?.into_iter());
        out.extend(TournamentInstitution::get_many_tournaments(db, &self.tournament_institutions.iter().collect()).await?.into_iter());
        out.extend(ParticipantClash::get_many_tournaments(db, &self.participant_clashes.iter().collect()).await?.into_iter());

        Ok(out)
    }

    pub async fn save_all<C>(&self, db: &C) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        self.save_all_with_options(db, false).await
    }

    pub async fn save_all_with_options<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        Tournament::save_many(db, guarantee_insert, &self.tournaments.iter().collect()).await?;
        TournamentInstitution::save_many(db, guarantee_insert, &self.tournament_institutions.iter().collect()).await?;
        Team::save_many(db, guarantee_insert, &self.teams.iter().collect()).await?;
        Participant::save_many(db, guarantee_insert, &self.participants.iter().collect()).await?;
        TournamentRound::save_many(db, guarantee_insert, &self.rounds.iter().collect()).await?;
        Ballot::save_many(db, guarantee_insert, &self.ballots.iter().collect()).await?;
        TournamentDebate::save_many(db, guarantee_insert, &self.debates.iter().collect()).await?;
        ParticipantClash::save_many(db, guarantee_insert, &self.participant_clashes.iter().collect()).await?;
        Ok(())
    }

    pub async fn save_all_and_log_for_tournament<C>(&self, db: &C, tournament_id: Uuid) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        self.save_all_with_options_and_log_for_tournament(db, false, tournament_id).await
    }

    pub async fn save_all_with_options_and_log_for_tournament<C>(&self, db: &C, guarantee_insert: bool, tournament_id: Uuid) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        self.save_all_with_options(db, guarantee_insert).await?;
        self.save_log_with_tournament_id(db, tournament_id).await?;

        Ok(())
    }

    pub fn into_entity_iterator(self) -> impl Iterator<Item=Entity> {
        self.participants.into_iter().map(|p| Entity::Participant(p))
        .chain(self.ballots.into_iter().map(|b| Entity::Ballot(b)))
        .chain(self.tournaments.into_iter().map(|t| Entity::Tournament(t)))
        .chain(self.rounds.into_iter().map(|r| Entity::TournamentRound(r)))
        .chain(self.debates.into_iter().map(|d| Entity::TournamentDebate(d)))
        .chain(self.teams.into_iter().map(|t| Entity::Team(t)))
        .chain(self.tournament_institutions.into_iter().map(|t| Entity::TournamentInstitution(t)))
        .chain(self.participant_clashes.into_iter().map(|p| Entity::ParticipantClash(p)))
    }

    pub fn get_entity_ids(&self) -> Vec<(String, Uuid)> {
        self.participants.iter().map(|p| ("Participant".to_string(), p.uuid.clone()))
        .chain(self.ballots.iter().map(|b| ("Ballot".to_string(), b.uuid.clone())))
        .chain(self.tournaments.iter().map(|t| ("Tournament".to_string(), t.uuid.clone())))
        .chain(self.rounds.iter().map(|r| ("TournamentRound".to_string(), r.uuid.clone())))
        .chain(self.debates.iter().map(|d| ("TournamentDebate".to_string(), d.uuid.clone())))
        .chain(self.teams.iter().map(|t| ("Team".to_string(), t.uuid.clone())))
        .chain(self.tournament_institutions.iter().map(|t| ("TournamentInstitution".to_string(), t.uuid.clone())))
        .chain(self.participant_clashes.iter().map(|p| ("ParticipantClash".to_string(), p.uuid.clone())))
        .collect_vec()
    }

    /* Saves all changes with a single tournament id. This function does not check whether all changes do belong to the same tournament.
     */
    pub async fn save_log_with_tournament_id<C>(&self, transaction: &C, tournament_id: Uuid) -> Result<Uuid, Box<dyn Error>> where C: ConnectionTrait {
        let last_log_entry = tournament_log::Entity::find()
        .filter(tournament_log::Column::TournamentId.eq(tournament_id))
        .order_by_desc(tournament_log::Column::SequenceIdx)
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

        let new_entries = self.get_entity_ids().into_iter().enumerate().map(|(idx, (name, uuid))| {
            tournament_log::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                timestamp: ActiveValue::Set(chrono::offset::Local::now().naive_local()),
                sequence_idx: ActiveValue::Set(last_sequence_idx + idx as i32),
                tournament_id: ActiveValue::Set(tournament_id),
                target_type: ActiveValue::Set(name),
                target_uuid: ActiveValue::Set(uuid)
            }
        }).collect_vec();

        if new_entries.len() > 0 {
            log_head = new_entries[new_entries.len() - 1].uuid.clone().unwrap();
            tournament_log::Entity::insert_many(new_entries).exec(transaction).await?;
        }

        Ok(log_head)
    }
}

impl From<Vec<Entity>> for EntityGroups {
    fn from(entities: Vec<Entity>) -> Self {
        let mut groups = EntityGroups::new();

        for e in entities {
            groups.add(e);
        }

        groups
    }
}

impl Entity {
    pub fn get_processing_order(&self) -> u64 {
        match self {
            Entity::Tournament(_) => 0,
            Entity::TournamentInstitution(_) => 1,
            Entity::Team(_) => 2,
            Entity::Participant(_) => 3,
            Entity::Ballot(_) => 4,
            Entity::TournamentRound(_) => 5,
            Entity::TournamentDebate(_) => 6,
            Entity::ParticipantClash(_) => 7,
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            Entity::Participant(_) => "Participant".to_string(),
            Entity::Ballot(_) => "Ballot".to_string(),
            Entity::Tournament(_) => "Tournament".to_string(),
            Entity::TournamentRound(_) => "TournamentRound".to_string(),
            Entity::TournamentDebate(_) => "TournamentDebate".to_string(),
            Entity::Team(_) => "Team".to_string(),
            Entity::TournamentInstitution(_) => "TournamentInstitution".to_string(),
            Entity::ParticipantClash(_) => "ParticipantClash".to_string(),
        }
    }

    pub fn get_uuid(&self) -> Uuid {
        match self {
            Entity::Participant(p) => p.uuid,
            Entity::Ballot(b) => b.uuid,
            Entity::Tournament(e) => e.uuid,
            Entity::TournamentRound(e) => e.uuid,
            Entity::TournamentDebate(e) => e.uuid,
            Entity::Team(e) => e.uuid,
            Entity::TournamentInstitution(e) => e.uuid,
            Entity::ParticipantClash(e) => e.uuid,
        }
    }
}

impl PartialOrd for Entity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        u64::cmp(&self.get_processing_order(), &other.get_processing_order())
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum EntityId {
    Participant(Uuid),
    Ballot(Uuid),
}

impl EntityId {
    pub fn from_type_and_id(target_type: &str, target_uuid: Uuid) -> EntityId {
        match target_type {
            "Participant" => EntityId::Participant(target_uuid),
            "Ballot" => EntityId::Ballot(target_uuid),
            _ => panic!("Unknown entity type {}", target_type)
        }
    }
}


pub async fn get_changed_entities_from_log<C>(transaction: &C, log_entries: Vec<crate::schema::tournament_log::Model>) -> Result<Vec<Entity>, Box<dyn Error>> where C: ConnectionTrait {
    let mut to_query : HashMap<String, Vec<Uuid>> = HashMap::new();
    log_entries.into_iter().for_each(|e| {
        match to_query.get_mut(&e.target_type) {
            Some(v) => {
                v.push(e.target_uuid);
            },
            None => {
                to_query.insert(e.target_type, vec![(e.target_uuid)]);
            }
        }
    });

    // FIXME: This is unelegant
    let mut all_new_entities = Vec::new();

    for (type_, uuids) in to_query.into_iter() {
        let new_entities = match type_.as_str() {
            "Participant" => Participant::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::Participant(e)).collect_vec(),
            "Ballot" => Ballot::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::Ballot(e)).collect_vec(),
            "Tournament" => Tournament::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::Tournament(e)).collect_vec(),
            "TournamentRound" => TournamentRound::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::TournamentRound(e)).collect_vec(),
            "TournamentDebate" => TournamentDebate::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::TournamentDebate(e)).collect_vec(),
            "Team" => Team::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::Team(e)).collect_vec(),
            "TournamentInstitution" => TournamentInstitution::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::TournamentInstitution(e)).collect_vec(),
            "ParticipantClash" => ParticipantClash::get_many(transaction, uuids).await?.into_iter().map(|e| Entity::ParticipantClash(e)).collect_vec(),
            _ => panic!("Unknown entity type {}", type_)
        };
        all_new_entities.extend(new_entities);
    };
    Ok(all_new_entities)
}
