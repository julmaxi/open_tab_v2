use std::{error::Error, f32::consts::E};

use itertools::Itertools;
use serde::{Serialize, Deserialize};
use sea_orm::{prelude::*, QueryOrder, QuerySelect, ActiveValue};

use crate::{domain::{participant::Participant, ballot::Ballot, TournamentEntity, tournament::Tournament, debate::TournamentDebate, round::TournamentRound, team::Team}, schema::tournament_log};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Entity {
    Tournament(Tournament),
    Team(Team),
    Participant(Participant),
    TournamentRound(TournamentRound),
    Ballot(Ballot),
    TournamentDebate(TournamentDebate),
}


pub struct EntityGroups {
    tournaments: Vec<Tournament>,
    rounds: Vec<TournamentRound>,
    debates: Vec<TournamentDebate>,
    participants: Vec<Participant>,
    ballots: Vec<Ballot>,
    teams: Vec<Team>,
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
        }
    }

    pub async fn get_all_tournaments<C>(&self, db: &C) -> Result<Vec<Option<Uuid>>, Box<dyn Error>> where C: ConnectionTrait {
        let mut out = Vec::new();

        out.extend(Participant::get_many_tournaments(db, &self.participants.iter().collect()).await?.into_iter());
        out.extend(Ballot::get_many_tournaments(db, &self.ballots.iter().collect()).await?.into_iter());

        Ok(out)
    }

    pub async fn save_all<C>(&self, db: &C) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        Tournament::save_many(db, false, &self.tournaments.iter().collect()).await?;
        Team::save_many(db, false, &self.teams.iter().collect()).await?;
        Participant::save_many(db, false, &self.participants.iter().collect()).await?;
        TournamentRound::save_many(db, false, &self.rounds.iter().collect()).await?;
        Ballot::save_many(db, false, &self.ballots.iter().collect()).await?;
        TournamentDebate::save_many(db, false, &self.debates.iter().collect()).await?;
        Ok(())   
    }

    pub fn into_entity_iterator(self) -> impl Iterator<Item=Entity> {
        self.participants.into_iter().map(|p| Entity::Participant(p))
        .chain(self.ballots.into_iter().map(|b| Entity::Ballot(b)))
        .chain(self.tournaments.into_iter().map(|t| Entity::Tournament(t)))
        .chain(self.rounds.into_iter().map(|r| Entity::TournamentRound(r)))
        .chain(self.debates.into_iter().map(|d| Entity::TournamentDebate(d)))
        .chain(self.teams.into_iter().map(|t| Entity::Team(t)))
    }

    /* Saves all changes with a single tournament id. This function does not check whether all changes do belong to the same tournament.
     */
    pub async fn save_log_with_tournament_id<C>(self, transaction: &C, tournament_id: Uuid) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        let last_log_entry = tournament_log::Entity::find()
        .filter(tournament_log::Column::TournamentId.eq(tournament_id))
        .order_by_desc(tournament_log::Column::SequenceIdx)
        .limit(1)
        .one(transaction)
        .await?;

        let last_sequence_idx = match last_log_entry {
            Some(entry) => entry.sequence_idx,
            None => 0,
        };
    
    
        let new_entries = self.into_entity_iterator().enumerate().map(|(idx, e)| {
            tournament_log::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                timestamp: ActiveValue::Set(chrono::offset::Local::now().naive_local()),
                sequence_idx: ActiveValue::Set(last_sequence_idx + idx as i32),
                tournament_id: ActiveValue::Set(tournament_id),
                target_type: ActiveValue::Set(e.get_name()),
                target_uuid: ActiveValue::Set(e.get_uuid())
            }
        }).collect_vec();
    
        tournament_log::Entity::insert_many(new_entries).exec(transaction).await?;
            
        Ok(())
    }
}

impl Entity {
    pub fn get_processing_order(&self) -> u64 {
        match self {
            Entity::Tournament(_) => 0,
            Entity::Team(_) => 1,
            Entity::Participant(_) => 2,
            Entity::Ballot(_) => 3,
            Entity::TournamentRound(_) => 4,
            Entity::TournamentDebate(_) => 5,
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