#[macro_use] extern crate rocket;
use std::collections::hash_map::RandomState;
use std::path::Path;
use std::{collections::HashMap, error::Error};
use open_tab_entities::prelude::SpeechRole;
use open_tab_entities::{Entity, EntityGroups, EntityId, get_changed_entities_from_log, domain};
use open_tab_entities::domain::{ballot::Ballot, participant::Participant, TournamentEntity};
use open_tab_entities::schema::{self, tournament_log, tournament};
use rocket::fs::{FileServer, relative, NamedFile};
use rocket::futures::TryFutureExt;
use rocket::http::hyper::body::HttpBody;
use rocket::response::status::Custom;
use rocket::serde::{Deserialize, Serialize, json::Json};
use migration::{MigratorTrait, Query, JoinType};
use rocket::State;
use rocket_dyn_templates::{Template, context};
use sea_orm::{prelude::*, Database, ConnectionTrait, DbBackend, Statement, QuerySelect, QueryOrder, TransactionTrait, ActiveValue, QueryTrait};
use itertools::Itertools;
use rocket::http::Status;
use rocket::{Rocket, Build};
use log::{info};

use open_tab_server::{TournamentUpdate, TournamentUpdateResponse, TournamentChanges, handle_error, handle_error_dyn};


#[post("/tournament/<tournament_id>/update", data="<updates>")]
async fn update_tournament(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid, updates: Json<TournamentUpdate>) -> Result<String, Custom<String>> {
    let transaction = db.begin_with_config(Some(sea_orm::IsolationLevel::Serializable), None).await.map_err(handle_error)?;

    if let Some(expected_log_sequence_number) = updates.expected_log_head {
        let latest_log = tournament_log::Entity::find()
            .filter(tournament_log::Column::TournamentId.eq(tournament_id))
            .order_by_desc(tournament_log::Column::SequenceIdx)
            .limit(1)
            .one(&transaction)
            .await
            .map_err(handle_error)?;
        if let Some(latest_log) = latest_log {
            if latest_log.uuid != expected_log_sequence_number {
                return Err(Custom(Status::Conflict, "Expected log is not in sequence".into()))
            }
        }
        else if !expected_log_sequence_number.is_nil() {
            return Err(Custom(Status::Conflict, "Expected log sequence number is not current log sequence".into()))
        }
    }

    let existing_versions : Vec<(Uuid,)> = tournament_log::Entity::find()
        .select_only()
        .column(tournament_log::Column::Uuid)
        .filter(tournament_log::Column::TournamentId.eq(tournament_id).and(
            tournament_log::Column::Uuid.is_in(
                updates.changes.iter().map(|e| e.version).collect_vec()
            )
        ))
        .into_tuple()
        .all(&transaction)
        .await
        .map_err(handle_error)?;
    
    let mut updates : TournamentUpdate = updates.into_inner();

    updates.changes = updates.changes.into_iter().filter(|e| !existing_versions.contains(&(e.version,))).collect_vec();
    updates.changes.sort_by_key(|e| e.entity.get_processing_order());

    let mut changeset = EntityGroups::new();
    let mut prev_change = None;
    for change in updates.changes.into_iter() {
        let identity = Some((change.entity.get_name(), change.entity.get_uuid()));
        if identity == prev_change {
            return Err(Custom(Status::BadRequest, "Duplicate entity".into()))
        }
        prev_change = identity;
        changeset.add_versioned(change.entity, change.version);
    }

    changeset.save_all(&transaction).await.map_err(handle_error_dyn)?;

    // Entity tournament ids can be a bit tricky to determine, since
    // they might be multiple layers deep. To avoid having to replicate
    // a lot of what the database can do better anyway, we first
    // write the changes and then rollback if any of the entity ids
    // do not match
    let tournament_uuids = changeset.get_all_tournaments(&transaction).await.map_err(handle_error_dyn)?;
    if tournament_uuids.iter().any(|u| *u != Some(tournament_id)) {
        transaction.rollback().await.map_err(handle_error)?;
        return Err(Custom(Status::BadRequest, "Entity tournament id does not match tournament id in path".into()))
    }

    let new_log_head = changeset.save_log_with_tournament_id(&transaction, tournament_id).await.map_err(handle_error_dyn)?;

    transaction.commit().await.map_err(handle_error)?;

    Ok(serde_json::to_string(&TournamentUpdateResponse{new_log_head }).map_err(handle_error)?)
}


#[derive(Debug, Serialize, Deserialize)]
struct SerializedTournamentLogEntry {
    sequence_idx: u64,
    #[serde(flatten)]
    entity: EntityId,
    log_uuid: Uuid,
}


impl From<tournament_log::Model> for SerializedTournamentLogEntry {
    fn from(model: tournament_log::Model) -> SerializedTournamentLogEntry {
        SerializedTournamentLogEntry {
            sequence_idx: model.sequence_idx as u64,
            log_uuid: model.uuid,
            entity: EntityId::from_type_and_id(&model.target_type, model.target_uuid),
        }
    }
}

#[get("/tournament/<tournament_id>/log")]
async fn get_tournament_log(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid) -> Result<Json<Vec<SerializedTournamentLogEntry>>, Custom<String>> {
    get_tournament_log_since(db, tournament_id, Uuid::nil()).await
}

#[get("/tournament/<tournament_id>/log?<since>")]
async fn get_tournament_log_since(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid, since: Uuid) -> Result<Json<Vec<SerializedTournamentLogEntry>>, Custom<String>> {
    // TODO: This can be a single select
    let start_idx = if since == Uuid::nil() {
        0
    }
    else {
        schema::tournament_log::Entity::find_by_id(
            since
        ).one(
            db.inner()
        ).await.map_err(handle_error)?
        .map(|e| e.sequence_idx + 1).ok_or(Custom(Status::NotFound, "Log entry not found".to_string()))?
    };

    let log_entries = schema::tournament_log::Entity::find().filter(
        schema::tournament_log::Column::TournamentId.eq(tournament_id).and(
            schema::tournament_log::Column::SequenceIdx.gte(start_idx)
        )
    ).all(db.inner()).await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;

    let out = log_entries.into_iter().map(|e| SerializedTournamentLogEntry::from(e)).collect_vec();

    Ok(Json(out))
}

#[get("/tournament/<tournament_id>/changes")]
async fn get_tournament_changes(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid) -> Result<Json<TournamentChanges>, Custom<String>> {
    get_tournament_changes_since(db, tournament_id, Uuid::nil()).await
}

#[get("/tournament/<tournament_id>/changes?<since>")]
async fn get_tournament_changes_since(db: &State<DatabaseConnection>, tournament_id: rocket::serde::uuid::Uuid, since: Uuid) -> Result<Json<TournamentChanges>, Custom<String>> {
    // TODO: This can be a single select
    let start_idx = if since == Uuid::nil() {
        0
    }
    else {
        schema::tournament_log::Entity::find_by_id(
            since
        ).one(
            db.inner()
        ).await.map_err(handle_error)?
        .map(|e| e.sequence_idx + 1).ok_or(Custom(Status::NotFound, "Log entry not found".to_string()))?
    };

    let transaction = db.begin().await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;
    
    let log_entries = schema::tournament_log::Entity::find().filter(
        schema::tournament_log::Column::TournamentId.eq(tournament_id).and(
            schema::tournament_log::Column::SequenceIdx.gte(start_idx)
        )
    ).order_by_asc(schema::tournament_log::Column::SequenceIdx)
    .all(&transaction).await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;

    let log_head = log_entries.last().map(|e| e.uuid).unwrap_or(Uuid::nil());

    let mut latest_versions = HashMap::new();
    for entry in log_entries.into_iter() {
        latest_versions.insert((entry.target_type.clone(), entry.target_uuid), entry);
    }

    let all_new_entities = get_changed_entities_from_log(&transaction, latest_versions.into_values().into_iter().collect()).map_err(|_| Custom(Status::InternalServerError, "Error".to_string())).await?;
    transaction.commit().await.map_err(|_| Custom(Status::InternalServerError, "Error".to_string()))?;

    Ok(Json(
        TournamentChanges {
            changes: all_new_entities,
            log_head
        }
    ))
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


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ParticipantHomePageInfo {
    name: String,
    team_name: Option<String>,
    role: String
}


struct ParticipantHomePageRoundInfo {
    round_number: u32,
    room_index: u32,
    role: ParticipantHomePageRoundRoleInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ParticipantHomePageRoundRoleInfo {
    TeamSpeaker { team_role: SpeechRole },
    NonAlignedSpeaker { position: u8 },
    Adjudicator { room_idx: u32, is_chair: bool },
    Unknown
}


#[get("/home/<participant_uuid>")]
async fn participant_homepage(participant_uuid: Uuid, db: &State<DatabaseConnection>) -> Result<Template, Custom<String>> {
    let participant = domain::participant::Participant::get_many(
        db.inner(),
        vec![participant_uuid]
    ).await.map_err(|e| match e {
        domain::participant::ParticipantParseError::ParticipantDoesNotExist => Custom(Status::NotFound, "Participant not found".to_string()),
        _ => handle_error(e)
    })?.into_iter().next().expect("List was empty. Apparently uuid missing error failed.");

    let rounds = domain::round::TournamentRound::get_all_in_tournament(db.inner(), participant.tournament_id).await.map_err(handle_error)?;

    let mut info = ParticipantHomePageInfo {..Default::default()};
    info.name = participant.name.clone();

    // FIXME: Just for testing this takes the first round. Should be the last.
    let current_active_round = schema::prelude::TournamentRound::find().filter(
        schema::tournament_round::Column::TournamentId.eq(participant.tournament_id)
    ).order_by_asc(schema::tournament_round::Column::Index)
    .limit(1)
    .one(db.inner()).await.map_err(handle_error)?;

    if let Some(current_active_round) = current_active_round {
        match participant.role {
            open_tab_entities::prelude::ParticipantRole::Speaker(speaker) => {
                if let Some(team_id) = speaker.team_id {
                    let team = schema::team::Entity::find_by_id(team_id).one(db.inner()).await.map_err(handle_error)?;

                    if let Some(team) = team {
                        info.team_name = Some(team.name);
                        let relevant_debates = schema::tournament_debate::Entity::find()
                        .inner_join(schema::ballot::Entity)
                        .filter(
                            schema::tournament_debate::Column::RoundId.eq(current_active_round.uuid).and(
                                schema::ballot::Column::Uuid.in_subquery(
                                    Query::select()
                                    .column(schema::ballot_speech::Column::BallotId)
                                    .from(schema::ballot_speech::Entity)
                                    .and_where(
                                        schema::ballot_speech::Column::SpeakerId.eq(participant.uuid)
                                    )
                                    .to_owned()
                                ).or(
                                    schema::ballot::Column::Uuid.in_subquery(
                                        Query::select()
                                        .column(schema::ballot_team::Column::BallotId)
                                        .from(schema::ballot_team::Entity)
                                        .and_where(
                                            schema::ballot_team::Column::TeamId.eq(team.uuid)
                                        )
                                        .to_owned()
                                    )
                                )
                            )
                        ).all(db.inner()).await.map_err(handle_error)?;
                    }
                }
            },
            open_tab_entities::prelude::ParticipantRole::Adjudicator(adj) => {

            },
        }
    }

    Ok(Template::render("participant_home", context!{participant: info}))
}


#[get("/debate/<debate_uuid>/ballot")]
async fn get_ballot_submission_form(debate_uuid: Uuid) -> Result<rocket::response::content::RawHtml<String>, Custom<String>> {
    let path = Path::new("static/ballot_submission_form/index.html");
    Ok(rocket::response::content::RawHtml(std::fs::read_to_string(path).map_err(handle_error)?))
}


async fn config_rocket(db_config: DatabaseConfig) -> rocket::Rocket<rocket::Build> {
    let db = set_up_db(db_config).await.unwrap();
    migration::Migrator::up(&db, None).await.unwrap();

    let groups = open_tab_entities::mock::make_mock_tournament();
    dbg!(groups.debates[0].uuid);
    let e = groups.save_all(&db).await;

    rocket::build()
        .attach(Template::fairing())
        .manage(db)
        .mount("/static", FileServer::from(relative!("static")))
        .mount("/", routes![
            get_tournament_overview,
            update_tournament,
            get_tournament_log,
            get_tournament_changes,
            get_tournament_changes_since,
            participant_homepage,
            get_ballot_submission_form
        ]).mount(
            "/api/v1/",
            open_tab_server::ballots::routes()
        )
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
    use open_tab_entities::{schema, VersionedEntity};
    use open_tab_server::TournamentChanges;
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
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );
        
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![
                    VersionedEntity { entity: change, version: Uuid::from_u128(1) }
                ], ..Default::default()}
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
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );
        let change2 = Entity::Participant(
            Participant {
                uuid: Uuid::from_u128(201),
                name: "Test 2".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![
                    VersionedEntity { entity: change1, version: Uuid::from_u128(1) },
                    VersionedEntity { entity: change2, version: Uuid::from_u128(2) }
                ], ..Default::default()}
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
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );
        let change2 = Entity::Participant(
            Participant {
                uuid: Uuid::from_u128(201),
                name: "Test 2".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
                tournament_id: Uuid::from_u128(2),
                institutions: vec![]
            }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![
                    VersionedEntity { entity: change1, version: Uuid::from_u128(1) },
                    VersionedEntity { entity: change2, version: Uuid::from_u128(2) }
                ], ..Default::default()}
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
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );
        let change2 = Entity::Participant(
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test 2".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![
                    VersionedEntity { entity: change1, version: Uuid::from_u128(1) },
                    VersionedEntity { entity: change2, version: Uuid::from_u128(2) }
                ], ..Default::default()}
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
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default()}),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );

        let change = VersionedEntity { entity: change, version: Uuid::from_u128(1) };
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change.clone()], ..Default::default()}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);
        
        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/changes").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : TournamentChanges = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

        assert_eq!(result.changes.len(), 1);
        assert_eq!(&result.changes[0], &change);
    }

    #[rocket::async_test]
    async fn test_changes_include_only_one_change_per_entity() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change = Entity::Participant(
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default()}),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );

        let change = VersionedEntity { entity: change, version: Uuid::from_u128(1) };
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change.clone()], ..Default::default()}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);
        
        let change = Entity::Participant(
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test2".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default()}),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );

        let change = VersionedEntity { entity: change, version: Uuid::from_u128(2) };
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change.clone()], ..Default::default()}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);

        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/changes").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : TournamentChanges = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
        dbg!(&result);

        assert_eq!(result.changes.len(), 1);
        assert_eq!(&result.changes[0], &change);
    }

    #[rocket::async_test]
    async fn test_duplicate_changes_are_ignored() {
        let rocket = setup_tournament_test().await;
        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        let change = Entity::Participant(
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default()}),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );

        let change = VersionedEntity { entity: change, version: Uuid::from_u128(1) };
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change.clone()], ..Default::default()}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);
        
        let change = Entity::Participant(
            Participant {
                uuid: Uuid::from_u128(200),
                name: "Test2".into(),
                role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {..Default::default()}),
                tournament_id: Uuid::from_u128(1),
                institutions: vec![]
            }
        );

        let change = VersionedEntity { entity: change, version: Uuid::from_u128(1) };
       
        let create_response = client.post("/tournament/00000000-0000-0000-0000-000000000001/update").body(
            serde_json::to_string(
                &TournamentUpdate{changes: vec![change.clone()], ..Default::default()}
            )
            .unwrap()
        ).dispatch().await;
        assert_eq!(create_response.status(), Status::Ok);


        let response = client.get("/tournament/00000000-0000-0000-0000-000000000001/changes").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let result : TournamentChanges = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

        assert_eq!(result.changes.len(), 1);
    }

}
