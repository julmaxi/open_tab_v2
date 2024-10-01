use std::collections::{HashMap, HashSet};

use migration::MigratorTrait;
use open_tab_entities::{domain::{debate_backup_ballot::DebateBackupBallot, entity::LoadEntity}, prelude::{Ballot, TournamentDebate, TournamentRound}, schema::{self, debate_backup_ballot}, EntityGroup, EntityGroupEntityTrait, EntityGroupTrait, EntityType};
use open_tab_server::sync::{get_entity_changes_since, get_log_since};
use sea_orm::{prelude::Uuid, ConnectionTrait, DbBackend, EntityTrait, IntoActiveModel, Statement};

//#[tokio::main]
async fn main_old() {
    let url = "postgres://open_tab@localhost/cd_ber_24_r3";

    let db = sea_orm::Database::connect(url).await.unwrap();

    db.execute(Statement::from_string(DbBackend::Postgres, "SET search_path TO public")).await.unwrap();

    let local_db = sea_orm::Database::connect("sqlite://./ber.sqlite3?mode=rwc").await.unwrap();

    let tournament_id = Uuid::parse_str("446de93a-67d2-4a7e-8e01-b7a1f16bf1a7").unwrap();

    let changes = get_entity_changes_since(&db, tournament_id, None).await.unwrap();

    let mut entities_to_save = vec![];
    for (entity_type, entities) in changes.entities.into_iter() {
        for entry in entities {
            entities_to_save.push(entry.current_value);
        }
    }
    migration::Migrator::up(&local_db, None).await.unwrap();


    let group = EntityGroup::from(entities_to_save);

    let deleted_tournamet_uuids = group.get_all_deletion_tournaments(&db).await.unwrap();
    if !deleted_tournamet_uuids.into_iter().all(|t| t == Some(tournament_id)) {
        println!("Rejecting push trying to delete in other tournaments");
    }
    group.save_all(&local_db).await.unwrap();

    let mut remote_log_models = changes.log.iter().enumerate().map(
        |(idx, entry)| {
            open_tab_entities::schema::tournament_log::Model {
                uuid: entry.uuid,
                tournament_id,
                target_type: entry.target_type.as_str().into(),
                target_uuid: entry.target_uuid,
                timestamp: entry.timestamp,
                sequence_idx: idx as i32 + 1
            }.into_active_model()
        }
    ).collect::<Vec<_>>();
    open_tab_entities::schema::tournament_log::Entity::insert_many(remote_log_models).exec(&local_db).await.unwrap();

}

#[tokio::main]
async fn main() {
    let url = "sqlite:///Users/juliussteen/Documents/open_tab_db.sqlite3";
    let db = sea_orm::Database::connect(url).await.unwrap();
    let tournament_id = Uuid::parse_str("446de93a-67d2-4a7e-8e01-b7a1f16bf1a7").unwrap();
    let changes = get_entity_changes_since(&db, tournament_id, None).await.unwrap();

    let mut entities_to_save = vec![];
    for (entity_type, entities) in changes.entities.into_iter() {
        for entry in entities {
            entities_to_save.push(entry.current_value);
        }
    }

    let mut ballot_ids = vec![];

    for e in entities_to_save.iter() {
        match e {
            open_tab_entities::EntityState::Exists(e) => {
                let uuid = e.get_uuid();
                let t = EntityGroup::from(vec![e.clone()]);
                let ts = t.get_all_tournaments(&db).await.unwrap();
                //println!("{:?} {} {:?}", e.get_type(), uuid, ts);

                if e.get_type() == EntityType::Ballot {
                    ballot_ids.push(uuid);
                }

                if e.get_type() == EntityType::Ballot {
                    let t = EntityGroup::from(vec![e.clone()]);
                    let ts = t.get_all_tournaments(&db).await.unwrap();    
                }
            }
            open_tab_entities::EntityState::Deleted { uuid, type_ } => {
                //println!("Deleted {:?} {}", type_, uuid)
            },
        }
    }

    let rounds = TournamentRound::get_all_in_tournament(&db, tournament_id).await.unwrap();
    let round_ids = rounds.iter().map(|r| r.uuid).collect::<Vec<_>>();

    let b = schema::ballot::Entity::find().all(&db).await.unwrap();
    let all_ballots = Ballot::get_many(&db, b.into_iter().map(|b| b.uuid).collect::<Vec<_>>()).await.unwrap();

    let all_ballots = all_ballots.into_iter().map(|b| b.uuid).collect::<Vec<_>>();

    let backup_ballots = debate_backup_ballot::Entity::find().all(&db).await.unwrap();

    let debates = TournamentDebate::get_all_in_rounds(&db, round_ids).await.unwrap().into_iter().flatten().collect::<Vec<_>>();

    let mut ballots = HashMap::new();

    for b in backup_ballots.iter() {
        ballots.insert(b.ballot_id, "backup");
    }

    for d in debates.iter() {
        ballots.insert(d.ballot_id, "orig");
    }

    let mut missing_cnt = 0;
    for b in ballot_ids.iter() {
        if !ballots.contains_key(b) {
            println!("Missing ballot {}", b);
            missing_cnt += 1;
        }
    }

    println!("Missing ballots: {}", missing_cnt);
}