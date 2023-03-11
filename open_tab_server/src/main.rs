#[macro_use] extern crate rocket;
use std::{collections::HashMap, error::Error};
use open_tab_entities::{Entity, EntityGroups, EntityId};
use open_tab_entities::domain::{ballot::Ballot, participant::Participant, TournamentEntity};
use open_tab_entities::schema::{self, tournament_log, tournament};
use rocket::futures::TryFutureExt;
use rocket::response::status::Custom;
use rocket::serde::{Deserialize, Serialize, json::Json};
use migration::MigratorTrait;
use rocket::State;
use sea_orm::{prelude::*, Database, ConnectionTrait, DbBackend, Statement, QuerySelect, QueryOrder, TransactionTrait, ActiveValue, QueryTrait};
use itertools::Itertools;
use rocket::http::Status;
use rocket::{Rocket, Build};
use log::{info};

#[derive(Serialize, Deserialize)]
struct TournamentUpdate {
    changes: Vec<Entity>
}



#[post("/tournament/<tournament_id>/update", data="<updates>")]
async fn update_tournament(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid, updates: Json<TournamentUpdate>) -> Result<&'static str,
 Custom<&'static str>> {
    let transaction = db.begin_with_config(Some(sea_orm::IsolationLevel::Serializable), None).await.map_err(|_| Custom(Status::InternalServerError, "Error"))?;
    let mut updates : TournamentUpdate = updates.into_inner();

    updates.changes.sort_by_key(|e| e.get_processing_order());

    let mut changeset = EntityGroups::new();
    let mut prev_change = None;
    for change in updates.changes.into_iter() {
        let identity = Some((change.get_name(), change.get_uuid()));
        if identity == prev_change {
            return Err(Custom(Status::BadRequest, "Duplicate entity"))
        }
        prev_change = identity;
        changeset.add(change)
    }

    let tournament_uuids = changeset.get_all_tournaments(&transaction).await.map_err(|_| Custom(Status::InternalServerError, "Error"))?;

    if tournament_uuids.iter().any(|u| *u != Some(tournament_id)) {
        return Err(Custom(Status::BadRequest, "Entity tournament id does not match tournament id in path"))
    }

    changeset.save_all(&transaction).await.map_err(|_| Custom(Status::InternalServerError, "Error"))?;
    changeset.save_log_with_tournament_id(&transaction, tournament_id).await.map_err(|_| Custom(Status::InternalServerError, "Error"))?;

    transaction.commit().await.map_err(|_| Custom(Status::InternalServerError, "Error"))?;

    Ok("Done")
}



#[derive(Debug, Serialize, Deserialize)]
struct SerializedTournamentLogEntry {
    sequence_idx: u64,
    #[serde(flatten)]
    entity: EntityId,
}


impl From<tournament_log::Model> for SerializedTournamentLogEntry {
    fn from(model: tournament_log::Model) -> SerializedTournamentLogEntry {
        SerializedTournamentLogEntry {
            sequence_idx: model.sequence_idx as u64,
            entity: EntityId::from_type_and_id(&model.target_type, model.target_uuid),
        }
    }
}

#[get("/tournament/<tournament_id>/log")]
async fn get_tournament_log(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid) -> Result<Json<Vec<SerializedTournamentLogEntry>>, Custom<String>> {
    get_tournament_log_since(db, tournament_id, 0).await
}

#[get("/tournament/<tournament_id>/log?<since>")]
async fn get_tournament_log_since(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid, since: u64) -> Result<Json<Vec<SerializedTournamentLogEntry>>, Custom<String>> {
    let log_entries = schema::tournament_log::Entity::find().filter(
        schema::tournament_log::Column::TournamentId.eq(tournament_id).and(
            schema::tournament_log::Column::SequenceIdx.gte(since)
        )
    ).all(db.inner()).await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;

    let out = log_entries.into_iter().map(|e| SerializedTournamentLogEntry::from(e)).collect_vec();

    Ok(Json(out))
}

#[get("/tournament/<tournament_id>/changes")]
async fn get_tournament_changes(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid) -> Result<Json<Vec<Entity>>, Custom<String>> {
    get_tournament_changes_since(db, tournament_id, 0).await
}

#[get("/tournament/<tournament_id>/changes?<since>")]
async fn get_tournament_changes_since(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid, since: u64) -> Result<Json<Vec<Entity>>, Custom<String>> {
    let transaction = db.begin().await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;
    
    let log_entries = schema::tournament_log::Entity::find().filter(
        schema::tournament_log::Column::TournamentId.eq(tournament_id).and(
            schema::tournament_log::Column::SequenceIdx.gte(since)
        )
    ).all(&transaction).await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;

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
            "Participant" => Participant::get_many(&transaction, uuids).await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?.into_iter().map(|e| Entity::Participant(e)).collect_vec(),
            "Ballot" => Ballot::get_many(&transaction, uuids).await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?.into_iter().map(|e| Entity::Ballot(e)).collect_vec(),
            _ => panic!("Unknown entity type {}", type_)
        };
        all_new_entities.extend(new_entities);
    };

    transaction.commit().await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;

    Ok(Json(all_new_entities))
}

 #[get("/tournament/<tournament_id>")]
 async fn get_tournament_overview(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid) -> Result<Option<&'static str>, Custom<&'static str>> {
    info!("{}", tournament_id);
    let t = schema::prelude::Tournament::find_by_id(tournament_id).one(db.inner()).await.map_err(|_| Custom(Status::InternalServerError, "Error"))?;
    Ok(t.map(|_| "Exists"))
 }
 

pub struct DatabaseConfig {
    url: String,
    name: String
}

impl DatabaseConfig {
    fn new(url: String, name: String) -> DatabaseConfig {
        DatabaseConfig { url, name }
    }
}

// Replace with your database URL and database name
const DATABASE_URL: &str = "sqlite://./server.sqlite3?mode=rwc";
const DB_NAME: &str = "bakeries_db";

pub async fn set_up_db(config: DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(config.url.clone()).await?;

    let db = match db.get_database_backend() {
        DbBackend::MySql => {
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!("CREATE DATABASE IF NOT EXISTS `{}`;", config.name),
            ))
            .await?;

            let url = format!("{}/{}", DATABASE_URL, config.name);
            Database::connect(&url).await?
        }
        DbBackend::Postgres => {
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!("DROP DATABASE IF EXISTS \"{}\";", config.name),
            ))
            .await?;
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!("CREATE DATABASE \"{}\";", config.name),
            ))
            .await?;

            let url = format!("{}/{}", config.url, config.name);
            Database::connect(&url).await?
        }
        DbBackend::Sqlite => db,
    };

    Ok(db)
}


async fn config_rocket(db_config: DatabaseConfig) -> rocket::Rocket<rocket::Build> {
    let db = set_up_db(db_config).await.unwrap();
    migration::Migrator::up(&db, None).await.unwrap();
    rocket::build()
        .manage(db)
        .mount("/", routes![
            get_tournament_overview,
            update_tournament,
            get_tournament_log,
            get_tournament_changes,
            get_tournament_changes_since
        ])
}

#[launch]
async fn rocket() -> _ {
    config_rocket(DatabaseConfig::new(DATABASE_URL.into(), DB_NAME.into())).await
}


#[cfg(test)]
mod test {
    use crate::{DatabaseConfig, config_rocket, SerializedTournamentLogEntry, Entity, TournamentUpdate};

    use super::rocket;
    use open_tab_entities::domain::participant::{Participant, Adjudicator};
    use open_tab_entities::schema;
    use rocket::{State, Rocket, Build};
    use rocket::local::asynchronous::Client;
    use rocket::http::Status;
    use sea_orm::{DatabaseConnection, ActiveValue, prelude::Uuid, ActiveModelTrait};

    async fn test_rocket() -> Rocket<Build> {
        config_rocket(DatabaseConfig::new("sqlite::memory:".into(), "".into())).await
    }

    async fn setup_tournament_test() -> Rocket<Build> {
        let rocket = test_rocket().await;
        let db : &State<DatabaseConnection> = State::get(&rocket).unwrap();
        let db : &DatabaseConnection = &db;
        schema::tournament::ActiveModel {
            uuid: ActiveValue::Set(Uuid::from_u128(1)),            
        }.insert(db).await.unwrap();
        rocket
    }
    
    #[rocket::async_test]
    async fn test_get_tournament_overview() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_get_unknown_tournament_overview() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000010").dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn test_unmodified_tournament_log_is_empty() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/log").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : Vec<SerializedTournamentLogEntry> = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[rocket::async_test]
    async fn test_adding_participant_alters_log() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change = Entity::Participant(
            Participant { uuid: Uuid::from_u128(200), name: "Test".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
        
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change]}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);
        
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/log").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : Vec<SerializedTournamentLogEntry> = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

        assert_eq!(result.len(), 1);
        
    }

    #[rocket::async_test]
    async fn test_adding_many_participant_creates_individual_log_entries() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change1 = Entity::Participant(
            Participant { uuid: Uuid::from_u128(200), name: "Test".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
        let change2 = Entity::Participant(
            Participant { uuid: Uuid::from_u128(201), name: "Test 2".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change1, change2]}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);
        
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/log").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : Vec<SerializedTournamentLogEntry> = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

        assert_eq!(result.len(), 2);
    }

    #[rocket::async_test]
    async fn test_adding_participant_from_different_tournaments_is_rejected() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change1 = Entity::Participant(
            Participant { uuid: Uuid::from_u128(200), name: "Test".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
        let change2 = Entity::Participant(
            Participant { uuid: Uuid::from_u128(201), name: "Test 2".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(2) }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change1, change2]}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::BadRequest);        
    }

    #[rocket::async_test]
    async fn test_duplicate_entities_are_rejected() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change1 = Entity::Participant(
            Participant { uuid: Uuid::from_u128(200), name: "Test".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
        let change2 = Entity::Participant(
            Participant { uuid: Uuid::from_u128(200), name: "Test 2".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change1, change2]}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::BadRequest);
    }
    

    #[rocket::async_test]
    async fn test_changes_includes_full_data() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change = Entity::Participant(
            Participant { uuid: Uuid::from_u128(200), name: "Test".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change.clone()]}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);
        
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/changes").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : Vec<Entity> = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

        assert_eq!(result.len(), 1);

        assert_eq!(&result[0], &change);
    }

    #[rocket::async_test]
    async fn test_changes_include_only_one_change_per_entity() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change = Entity::Participant(
            Participant { uuid: Uuid::from_u128(200), name: "Test".into(), role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {}), tournament_id: Uuid::from_u128(1) }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change.clone()]}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);
        
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/changes").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : Vec<Entity> = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

        assert_eq!(result.len(), 1);

        assert_eq!(&result[0], &change);
    }
}
