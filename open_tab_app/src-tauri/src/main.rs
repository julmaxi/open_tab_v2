// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::{HashMap}, error::Error, fmt::{Display, Formatter}, time::Duration, iter::zip, borrow::BorrowMut, path::PathBuf, fs::{File, create_dir}, sync::PoisonError};

use migration::{MigratorTrait};
use open_tab_entities::{EntityGroup, domain::{tournament::Tournament, ballot::{SpeechRole, BallotParseError}, entity::LoadEntity, feedback_form::{FeedbackForm, FeedbackFormVisibility}, feedback_question::FeedbackQuestion}, schema::{self}, get_changed_entities_from_log, mock::{make_mock_tournament_with_options, MockOption}, utilities::BatchLoadError, EntityType};
use open_tab_server::{sync::{SyncRequestResponse, SyncRequest, FatLog, reconcile_changes, ReconciliationOutcome}, tournament::{CreateTournamentRequest, CreateTournamentResponse}, auth::{CreateUserRequest, CreateUserResponse}};
//use open_tab_server::{TournamentChanges};
use reqwest::Client;
use sea_orm::{prelude::*, Statement, Database, DatabaseTransaction, TransactionTrait, QueryOrder, IntoActiveModel, ActiveValue, QuerySelect};
use tauri::{async_runtime::block_on, State, AppHandle, Manager};
use open_tab_entities::prelude::*;
use itertools::{Itertools};
use serde::{Serialize, Deserialize};

use open_tab_app_backend::{View, draw_view::{DrawBallot}, LoadedView, Action, import::CSVReaderConfig, draw::evaluation::{DrawIssue, DrawEvaluator}, tournament_status_view::{TournamentStatusView, LoadedTournamentStatusView}};

use tokio::{sync::Mutex, sync::RwLock};


fn make_default_feedback_form(tournament_id: Uuid) -> EntityGroup {
    let basic_questions = vec![
        FeedbackQuestion {
            uuid: Uuid::new_v4(),
            short_name: "skill".into(),
            full_name: "Wie würdest du insgesamt die Kompetenz dieser JurorIn bewerten?".into(),
            description: "".into(),
            question_config: open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion{config: open_tab_entities::domain::feedback_question::RangeQuestionConfig {
                min: 0,
                max: 100,
                orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                labels: vec![
                    (0, "Sehr schlecht".into()),
                    (100, "Sehr gut".into()),
                ] }
            },
            tournament_id: Some(tournament_id),
        },
    ];

    let team_questions = vec![
        FeedbackQuestion {
            uuid: Uuid::new_v4(),
            short_name: "team_level".into(),
            full_name: "Wie fandest du die Punkzahl, die du/ihr für eure Reden erhalten hast/habt?".into(),
            description: "Gib an, wie weit (in Punkten) die gegebene Punktzahl von der Punkzahl, die du für angemessen gehalten hättest.".into(),
            question_config: open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion{config: open_tab_entities::domain::feedback_question::RangeQuestionConfig {
                min: -21,
                max: 21,
                orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                labels: vec![
                    (-21, ">10 zu niedrig".into()),
                    (21, ">10 zu hoch".into()),
                ] }},
            tournament_id: Some(tournament_id),
        },
    ];

    let speaker_questions = vec![
        FeedbackQuestion {
            uuid: Uuid::new_v4(),
            short_name: "speech_level".into(),
            full_name: "Wie fandest du die Punkzahl, die du/ihr für eure Reden erhalten hast/habt?".into(),
            description: "Gib an, wie weit (in Punkten) die gegebene Punktzahl von der Punkzahl, die du für angemessen gehalten hättest. Für Teams nenne die größte Abweichung.".into(),
            question_config: open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion{config: open_tab_entities::domain::feedback_question::RangeQuestionConfig {
                min: -11,
                max: 11,
                orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                labels: vec![
                    (-11, ">10 zu niedrig".into()),
                    (11, ">10 zu hoch".into()),
                ] }
            },
            tournament_id: Some(tournament_id),
        },
    ];

    let feedback_questions = vec![
        FeedbackQuestion {
            uuid: Uuid::new_v4(),
            short_name: "feedback_level".into(),
            full_name: "Unabhängig von deiner/eurer eigenen Einschätzung der Punktzahl. Hat das Feedback deine/eure Punktzahl gut wiedergespiegelt?".into(),
            description: "".into(),
            question_config: open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion{config: open_tab_entities::domain::feedback_question::RangeQuestionConfig {
                min: 0,
                max: 100,
                orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                labels: vec![
                    (0, "Ganz und gar nicht".into()),
                    (100, "Voll und ganz".into()),
                ] }},
            tournament_id: Some(tournament_id),
        },
        FeedbackQuestion {
            uuid: Uuid::new_v4(),
            short_name: "feedback_overall".into(),
            full_name: "Wie würdest du insgesamt die Qualität des Feedbacks bewerten?".into(),
            description: "".into(),
            question_config: open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion{config: open_tab_entities::domain::feedback_question::RangeQuestionConfig {
                min: 0,
                max: 100,
                orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                labels: vec![
                    (0, "Sehr schlecht".into()),
                    (100, "Sehr gut".into()),
                ] }},
            tournament_id: Some(tournament_id),
        },
    ];

    let wing_questions = vec![
        FeedbackQuestion {
            uuid: Uuid::new_v4(),
            short_name: "moderation".into(),
            full_name: "Wie hat er Chair die Jurierdiskussion geleitet?".into(),
            description: "".into(),
            question_config: open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion{config: open_tab_entities::domain::feedback_question::RangeQuestionConfig {
                min: 0,
                max: 100,
                orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                labels: vec![
                    (0, "Sehr schlecht".into()),
                    (100, "Sehr gut".into()),
                ] }},
            tournament_id: Some(tournament_id),
        },
    ];

    let chair_questions = vec![
        FeedbackQuestion {
            uuid: Uuid::new_v4(),
            short_name: "participation".into(),
            full_name: "Hat der/die JurorIn sich konstruktiv an der Jurierdiskussion beteiligt?".into(),
            description: "".into(),
            question_config: open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion{config: open_tab_entities::domain::feedback_question::RangeQuestionConfig {
                min: 0,
                max: 100,
                orientation: open_tab_entities::domain::feedback_question::RangeQuestionOrientation::HighIsGood,
                labels: vec![
                    (0, "Sehr schlecht".into()),
                    (100, "Sehr gut".into()),
                ] }},
            tournament_id: Some(tournament_id),
        },
    ];

    let comments_questions = vec![FeedbackQuestion {
        uuid: Uuid::new_v4(),
        short_name: "comments".into(),
        full_name: "Comments".into(),
        description: "Diese Kommentare werden später für die Juror/In einsehbar sein.".into(),
        question_config: open_tab_entities::domain::feedback_question::QuestionType::TextQuestion,
        tournament_id: Some(tournament_id),
    }];

    let chair_form = FeedbackForm {
        uuid: Uuid::new_v4(),
        name: "Chair Feedback to Wings".to_string(),
        visibility: FeedbackFormVisibility {
            show_chairs_for_wings: true,
            ..Default::default()
        },
        tournament_id: Some(tournament_id),
        questions: itertools::chain!(
            basic_questions.iter().map(|q| q.uuid.clone()),
            chair_questions.iter().map(|q| q.uuid.clone()),
            comments_questions.iter().map(|q| q.uuid.clone()),
        ).collect(),
    };

    let wing_form = FeedbackForm {
        uuid: Uuid::new_v4(),
        name: "Wing Feedback to Chairs".to_string(),
        visibility: FeedbackFormVisibility {
            show_wings_for_chairs: true,
            ..Default::default()
        },
        tournament_id: Some(tournament_id),
        questions: itertools::chain!(
            basic_questions.iter().map(|q| q.uuid.clone()),
            wing_questions.iter().map(|q| q.uuid.clone()),
            comments_questions.iter().map(|q| q.uuid.clone()),
        ).collect(),
    };

    let team_form = FeedbackForm {
        uuid: Uuid::new_v4(),
        name: "Team Feedback to Chairs".to_string(),
        visibility: FeedbackFormVisibility {
            show_teams_for_chairs: true,
            ..Default::default()
        },
        tournament_id: Some(tournament_id),
        questions: itertools::chain!(
            basic_questions.iter().map(|q| q.uuid.clone()),
            team_questions.iter().map(|q| q.uuid.clone()),
            speaker_questions.iter().map(|q| q.uuid.clone()),
            feedback_questions.iter().map(|q| q.uuid.clone()),
            comments_questions.iter().map(|q| q.uuid.clone()),
        ).collect(),
    };

    let speaker_form = FeedbackForm {
        uuid: Uuid::new_v4(),
        name: "Non-Aligned Feedback to Chairs".to_string(),
        visibility: FeedbackFormVisibility {
            show_non_aligned_for_chairs: true,
            ..Default::default()
        },
        tournament_id: Some(tournament_id),
        questions: itertools::chain!(
            basic_questions.iter().map(|q| q.uuid.clone()),
            speaker_questions.iter().map(|q| q.uuid.clone()),
            feedback_questions.iter().map(|q| q.uuid.clone()),
            comments_questions.iter().map(|q| q.uuid.clone()),
        ).collect(),
    };

    let group = EntityGroup::from(
        itertools::chain!(
            itertools::chain!(
                basic_questions.into_iter(),
                chair_questions.into_iter(),
                wing_questions.into_iter(),
                team_questions.into_iter(),
                speaker_questions.into_iter(),
                feedback_questions.into_iter(),
                comments_questions.into_iter(),
            ).map(
                |e| Entity::FeedbackQuestion(e)
            ),
            vec![
                Entity::FeedbackForm(chair_form),
                Entity::FeedbackForm(wing_form),
                Entity::FeedbackForm(team_form),
                Entity::FeedbackForm(speaker_form),
            ].into_iter(),
        ).collect_vec()
    );

    group
}

async fn connect_db() -> Result<DatabaseConnection, DbErr> {
    connect_db_to_file(None).await
}

async fn connect_db_to_file(path: Option<PathBuf>) -> Result<DatabaseConnection, DbErr> {
    let db_string = path.map(|path| format!("sqlite:{}?mode=rwc", path.to_string_lossy())).unwrap_or("sqlite::memory:".into());
    let db = Database::connect(db_string).await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _ = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    let has_tournament = schema::tournament::Entity::find().one(&db).await?.is_some();

    if !has_tournament {
        let mut mock_data = make_mock_tournament_with_options(MockOption { deterministic_uuids: true, use_random_names: true, use_feedback: false, ..Default::default() });
        let tournament_uuid = mock_data.tournaments[0].uuid.clone();

        let second_tournament = Tournament {
            uuid: Uuid::from_u128(2),
            annoucements_password: None,
        };
        mock_data.add(Entity::Tournament(second_tournament));

        mock_data.save_all_with_options(&db, true).await.unwrap();
        mock_data.save_log_with_tournament_id(&db, tournament_uuid).await.unwrap();

        let g = make_default_feedback_form(tournament_uuid.clone());
        g.save_all_and_log_for_tournament(&db, tournament_uuid).await.unwrap();
    }
/*
    let mut user_id = None;

    
    match reqwest::Client::new().post("http://localhost:3000/api/users").json(
        &CreateUserRequest {
            password: "testpassword".to_string(),
        }
    ).send().await {
        Ok(r) => {
            let r : CreateUserResponse = r.json().await.unwrap();
            user_id = Some(r);
            dbg!("User created", &user_id);
        }
        _ => {
            dbg!("Err with user");
        }
    };

    match reqwest::Client::new().post("http://localhost:3000/api/tournaments").basic_auth(user_id.map(|u| u.uuid.to_string()).unwrap_or("".into()), "testpassword".into()).json(
        &CreateTournamentRequest {
            uuid: Uuid::from_u128(1),
            name: "Test Tournament".to_string(),
        }
    ).send().await {
        Ok(_) => {
            schema::tournament_remote::ActiveModel {
                uuid: sea_orm::ActiveValue::Set(Uuid::new_v4()),
                tournament_id: sea_orm::ActiveValue::Set(Uuid::from_u128(1)),
                url: sea_orm::ActiveValue::Set("localhost:3000".to_string()),
                last_known_change: sea_orm::ActiveValue::Set(None),
                last_synced_change: sea_orm::ActiveValue::Set(None)
            }.insert(&db).await.unwrap();
        }
        _ => {
            dbg!("Err");
        }
    }
     */

    Ok(db)
}


#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SubscriptionResponse {
    Success(String),
    Error(String)
}


#[derive(Debug)]
enum ViewCacheError {
    ViewLoadError
}

impl Display for ViewCacheError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ViewCacheError {}

pub struct ViewCache {
    cached_views: HashMap<View, Box<dyn LoadedView>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeNotification {
    pub view: View,
    pub updated_paths: HashMap<String, serde_json::Value>
}

impl ViewCache {
    pub fn new() -> Self {
        Self {
            cached_views: HashMap::new()
        }
    }

    pub async fn get_view_string<C>(&mut self, view: View, db: &C) -> Result<String, Box<dyn Error>> where C: ConnectionTrait {
        let loaded_view = self.get_view(view, db).await?;
        let view_str = loaded_view.view_string().await?;

        Ok(view_str)
    }

    pub async fn force_replace<C>(&mut self, view: View, new_values: Box<dyn LoadedView>, db: &C) -> Result<ChangeNotification, Box<dyn Error>> where C: ConnectionTrait {
        self.cached_views.insert(view.clone(), new_values);
        //FIXME: Hacky
        let view_string = self.get_view_string(view.clone(), db).await?;
        let value : serde_json::Value = serde_json::from_str(&view_string)?;
        Ok(ChangeNotification {
            view: view,
            updated_paths: HashMap::from([(".".into(), value)]),
        })
    }

    pub async fn update_and_get_changes(&mut self, db: &DatabaseTransaction, changes: &EntityGroup) -> Result<Vec<ChangeNotification>, Box<dyn Error>> {
        let mut out = vec![];
        for (view, loaded_view) in self.cached_views.iter_mut() {
            let changes = loaded_view.update_and_get_changes(db, changes).await?;
            if let Some(changes) = changes {
                out.push(ChangeNotification {
                    view: view.clone(),
                    updated_paths: changes
                });
            }
        };

        Ok(out)
    }

    pub async fn get_view<C>(&mut self, view: View, db: &C) -> Result<&Box<dyn LoadedView>, Box<dyn Error>> where C: ConnectionTrait {
        let is_loaded = self.cached_views.contains_key(&view);

        if !is_loaded {
            let loaded_view = view.load(db).await?;
            self.cached_views.insert(view.clone(), loaded_view);
        };
        let loaded_view = self.cached_views.get(&view).unwrap();

        Ok(loaded_view)
    }
}


#[tauri::command]
async fn subscribe_to_view(view: View, db: State<'_, DatabaseConnection>, view_cache: State<'_, Mutex<ViewCache>>) -> Result<SubscriptionResponse, ()> {
    // TODO: Handle and report load errors
    let mut view_cache = view_cache.lock().await;
    let view_text = view_cache.get_view_string(view.clone(), db.inner()).await;

    Ok(view_text.map(|text| {
        SubscriptionResponse::Success(text)
    }).unwrap_or_else(|err| {
        SubscriptionResponse::Error(err.to_string())
    }))
}


#[tauri::command]
async fn add_remote(db: State<'_, DatabaseConnection>) {
    
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionResponse {
    success: bool,
    message: Option<String>,
}

async fn execute_action_impl(action: Action, db: &DatabaseConnection, view_cache: &mut ViewCache) -> Result<Vec<ChangeNotification>, Box<dyn Error>> {
    let transaction = db.begin().await?;
    let changes = action.execute(&transaction).await?;
    changes.save_all(&transaction).await?;
    let tournament = changes.get_all_tournaments(&transaction).await?.into_iter().filter_map(|s| s).next().unwrap();
    changes.save_log_with_tournament_id(&transaction, tournament).await?;

    transaction.commit().await?;
    let transaction = db.begin().await?;

    let notifications = view_cache.update_and_get_changes(&transaction, &changes).await?;
    transaction.commit().await?;

    Ok(notifications)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangeNotificationSet {
    changes: Vec<ChangeNotification>
}


#[tauri::command]
async fn execute_action(app: AppHandle, action: Action, db: State<'_, DatabaseConnection>, view_cache: State<'_, Mutex<ViewCache>>) -> Result<ActionResponse, ()> {
    let mut view_cache = view_cache.lock().await;
    let result = execute_action_impl(action, db.inner(), &mut *view_cache).await;

    Ok(match result {
        Ok(notifications) => {
            // TODO: Handle this more gracefully
            app.emit_all("views-changed", ChangeNotificationSet {changes: notifications}).expect("Event send failed");
            ActionResponse {
                success: true,
                message: None
            }
        },
        Err(err) => {
            dbg!(&err);
            ActionResponse {
                success: false,
                message: Some(err.to_string())
            }
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TournamentListEntry {
    name: String,
    uuid: Uuid
}

impl From<schema::tournament::Model> for TournamentListEntry {
    fn from(model: schema::tournament::Model) -> Self {
        TournamentListEntry {
            name: "Test Tournament".into(),
            uuid: model.uuid
        }
    }
}

#[tauri::command]
async fn get_tournament_list(db: State<'_, DatabaseConnection>) -> Result<Vec<TournamentListEntry>, ()> {
    let tournaments = schema::tournament::Entity::find().all(db.inner()).await.map_err(|_| ())?;

    Ok(tournaments.into_iter().map(TournamentListEntry::from).collect())
}


enum SyncNotification {
    SuccessPush,
    SuccessPull,
    FailPush,
    FailPull,
    Alive
}

#[derive(Debug)]
enum SyncError {
    ReqwestError(reqwest::Error),
    DatabaseError(sea_orm::DbErr),
    Other(String)
}

impl From<reqwest::Error> for SyncError {
    fn from(err: reqwest::Error) -> Self {
        SyncError::ReqwestError(err)
    }
}

impl From<sea_orm::DbErr> for SyncError {
    fn from(err: sea_orm::DbErr) -> Self {
        SyncError::DatabaseError(err)
    }
}

impl From<Box<dyn Error>> for SyncError {
    fn from(err: Box<dyn Error>) -> Self {
        SyncError::Other(err.to_string())
    }
}

impl From<String> for SyncError {
    fn from(err: String) -> Self {
        SyncError::Other(err)
    }
}

impl From<&'static str> for SyncError {
    fn from(err: &'static str) -> Self {
        SyncError::Other(format!("{}", err))
    }
}

impl From<BallotParseError> for SyncError {
    fn from(err: BallotParseError) -> Self {
        SyncError::Other(format!("{}", err))
    }
}

impl From<BatchLoadError> for SyncError {
    fn from(err: BatchLoadError) -> Self {
        SyncError::Other(format!("{}", err))
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::ReqwestError(err) => write!(f, "Reqwest error: {}", err),
            SyncError::DatabaseError(err) => write!(f, "Database error: {}", err),
            SyncError::Other(err) => write!(f, "Other error: {}", err)
        }
    }
}

impl std::error::Error for SyncError {}

async fn auto_accept_ballots<C>(changes: &EntityGroup, db: &C) -> Result<Option<EntityGroup>, SyncError> where C: ConnectionTrait {
    if changes.debate_backup_ballots.is_empty() {
        return Ok(None);
    }
    println!("Accepting {} new ballots", changes.debate_backup_ballots.len());
    let debates_by_id = TournamentDebate::get_many(db, changes.debate_backup_ballots.iter().map(|b| b.debate_id).collect_vec()).await?.into_iter().map(|debate| (debate.uuid, debate)).collect::<HashMap<_, _>>();
    println!("Loaded {} debates", debates_by_id.len());
    let current_debate_ballots_by_id = Ballot::get_many(
        db,
        debates_by_id.values().map(|d| d.ballot_id).collect_vec()
    ).await?.into_iter().map(|ballot| (ballot.uuid, ballot)).collect::<HashMap<_, _>>();
    println!("Loaded {} debate ballots", current_debate_ballots_by_id.len());

    let new_ballots_by_id = Ballot::get_many(db, changes.debate_backup_ballots.iter().map(|b| b.ballot_id).collect_vec()).await?.into_iter().map(|ballot| (ballot.uuid, ballot)).collect::<HashMap<_, _>>();
    println!("Loaded {} backup ballots", current_debate_ballots_by_id.len());    

    let mut new_changes = EntityGroup::new();
    for new_backup_ballot in changes.debate_backup_ballots.iter() {
        let old_debate = debates_by_id.get(&new_backup_ballot.debate_id).unwrap();
        let old_ballot = current_debate_ballots_by_id.get(&old_debate.ballot_id).unwrap();
        let new_ballot = new_ballots_by_id.get(&new_backup_ballot.ballot_id).unwrap();

        if !old_ballot.is_scored()
        && old_ballot.government.team == new_ballot.government.team
        && old_ballot.opposition.team == new_ballot.opposition.team
        && old_ballot.speeches.iter().filter_map(|s| match s.role {
            SpeechRole::NonAligned => Some(s.speaker),
            _ => None
        }).collect_vec() == new_ballot.speeches.iter().filter_map(|s| match s.role {
            SpeechRole::NonAligned => Some(s.speaker),
            _ => None
        }).collect_vec()
         {
            new_changes.add(Entity::TournamentDebate(TournamentDebate {
                ballot_id: new_ballot.uuid,
                ..old_debate.clone()
            }));
        }
    };

    Ok(Some(new_changes))
}

async fn pull_remote_changes<C>(target_tournament_remote: &schema::tournament_remote::Model, client: &Client, db: &C, view_cache: &Mutex<ViewCache>, app_handle: &AppHandle) -> Result<Option<EntityGroup>, SyncError> where C: ConnectionTrait + TransactionTrait {
    let mut remote_url = format!("{}/api/tournament/{}/log", target_tournament_remote.url, target_tournament_remote.tournament_id);

    if let Some(last_common_ancestor) = target_tournament_remote.last_synced_change {
        remote_url = format!("{}?since={}", remote_url, last_common_ancestor);
    }

    let response = client.get(remote_url).send().await?;
    let remote_changes : FatLog<Entity, EntityType> = response.json().await?;

    if remote_changes.log.len() > 0 {
        dbg!("Integrating remote changes", remote_changes.log.len());
        let transaction = db.begin().await?;

        let outcome = reconcile_changes(&transaction, target_tournament_remote.tournament_id, remote_changes, target_tournament_remote.last_synced_change, open_tab_server::sync::MergeStrategy::AlwaysLocal, true).await?;

        match outcome {
            ReconciliationOutcome::Success { new_last_common_ancestor, entity_group } => {
                let update = schema::tournament_remote::ActiveModel {                
                    uuid: ActiveValue::Unchanged(target_tournament_remote.uuid),
                    last_synced_change: ActiveValue::Set(Some(new_last_common_ancestor)),
                    ..Default::default()
                };
                update.update(&transaction).await?;
                transaction.commit().await?;

                let entity_group = entity_group.unwrap();

                let transaction = db.begin().await?;
                let mut view_cache = view_cache.lock().await;
                let notifications = view_cache.update_and_get_changes(&transaction, &entity_group).await?;
                app_handle.emit_all("views-changed", ChangeNotificationSet {changes: notifications}).expect("Event send failed");
                transaction.rollback().await?;

                let transaction = db.begin().await?;
                let new_changes = auto_accept_ballots(&entity_group, &transaction).await?;
                if let Some(new_changes) = new_changes {
                    new_changes.save_all_and_log_for_tournament(&transaction, target_tournament_remote.tournament_id).await?;
                    transaction.commit().await?;
                    let transaction = db.begin().await?;
                    let notifications = view_cache.update_and_get_changes(&transaction, &new_changes).await?;
                    transaction.rollback().await?;
                    app_handle.emit_all("views-changed", ChangeNotificationSet {changes: notifications}).expect("Event send failed");
                }
                else {
                    transaction.rollback().await?;
                }

                return Ok(None);
            }
            ReconciliationOutcome::Reject => {
                transaction.rollback().await?;
                return Err(SyncError::Other("Reconciliation failed".to_string()));
            }
        }

    }

    Ok(None)
}

async fn try_push_changes<C>(target_tournament_remote: &schema::tournament_remote::Model, client: &Client, db: &C) -> Result<(), SyncError> where C: ConnectionTrait + TransactionTrait {
    let remote_url = format!("{}/api/tournament/{}/log", target_tournament_remote.url, target_tournament_remote.tournament_id);

    let transaction = db.begin().await?;

    let change_log = open_tab_server::sync::get_entity_changes_since(
        &transaction,
        target_tournament_remote.tournament_id,
        target_tournament_remote.last_synced_change,
    ).await?;

    transaction.rollback().await?;

    if change_log.log.len() == 0 {
        return Ok(());
    }
    dbg!("Pushing changes", change_log.log.len());

    let response = client.post(remote_url).json(&
        SyncRequest {
            log: change_log,
            last_common_ancestor: target_tournament_remote.last_synced_change
        }
    ).send().await?;
    
    if response.status() == 200 {
        let response = response.json::<SyncRequestResponse>().await?;
        match response.outcome {
            open_tab_server::sync::APIReconciliationOutcome::Success { new_last_common_ancestor } => {
                let transaction = db.begin().await?;
                dbg!(&new_last_common_ancestor);
    
                let update = schema::tournament_remote::ActiveModel {                
                    uuid: ActiveValue::Unchanged(target_tournament_remote.uuid),
                    last_synced_change: ActiveValue::Set(Some(new_last_common_ancestor)),
                    ..Default::default()
                };
                update.update(&transaction).await?;
    
                transaction.commit().await?;
            }
            open_tab_server::sync::APIReconciliationOutcome::Reject => {
                dbg!("Remote rejected changes");
            }
        };    
    }
    else {
        dbg!(response.text().await?);
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlatBallotEvaluationResult {
    government: Vec<DrawIssue>,
    opposition: Vec<DrawIssue>,
    non_aligned_speakers: Vec<Vec<DrawIssue>>,
    adjudicators: Vec<Vec<DrawIssue>>,
}

#[tauri::command]
async fn evaluate_ballots(db: State<'_, DatabaseConnection>, tournament_id: Uuid, round_id: Uuid, ballots: Vec<DrawBallot>, target_uuid: Uuid) -> Result<Vec<FlatBallotEvaluationResult>, ()> {
    let evaluator = DrawEvaluator::new_from_other_rounds(
        db.inner(),
        tournament_id,
        round_id,
    ).await.map_err(|_| ())?;

    let eval_results = ballots.iter().map(|b| evaluator.find_issues_in_ballot(b));

    Ok(zip(ballots.iter(), eval_results).map(
        |(b, r)| FlatBallotEvaluationResult {
            government: r.government_issues.into_iter().filter(
                |i| i.target.uuid() == target_uuid
            ).collect_vec(),
            opposition: r.opposition_issues.into_iter().filter(
                |i| i.target.uuid() == target_uuid
            ).collect_vec(),
            non_aligned_speakers: b.non_aligned_speakers.iter().map(|s| r.non_aligned_issues.get(&s.uuid).map(|i| i.clone()).unwrap_or(Vec::new()).into_iter().filter(|i| i.target.uuid() == target_uuid).collect_vec()).collect_vec(),
            adjudicators: b.adjudicators.iter().map(|s| r.adjudicator_issues.get(&s.adjudicator.uuid).map(|i| i.clone()).unwrap_or(Vec::new()).into_iter().filter(|i| i.target.uuid() == target_uuid).collect_vec()).collect_vec(),
        }
    ).collect())
}

#[tauri::command]
async fn get_settings(settings: State<'_, RwLock<AppSettings>>) -> Result<AppSettings, ()> {
    Ok(settings.inner().read().await.clone())
}

#[tauri::command]
async fn open_tournament(handle: AppHandle, client: State<'_, Client>, open_tournament_manager: State<'_, Mutex<OpenTournamentManager>>, db: State<'_, DatabaseConnection>, tournament_id: Uuid) -> Result<(), ()> {
    let tournament_window = tauri::WindowBuilder::new(
        &handle,
        &tournament_id.to_string(), /* the unique window label */
        tauri::WindowUrl::App("index.html".into())
    ).title(&tournament_id.to_string()).build().map_err(|_| ())?;
      //tournament_window.emit("select_tournament", tournament_id).map_err(|_| ())?;

    handle.get_window("main").map(|h|
        h.close()
    );

    let update_process = TournamentUpdateProcess {
        tournament_id: tournament_id,
        app_handle: handle.clone(),
        client: client.inner().clone(),
        sync_frequency: chrono::Duration::seconds(10)
    };

    let mut open_tournament_manager = open_tournament_manager.inner().lock().await;
    let open_tournament_manager = open_tournament_manager.borrow_mut();

    open_tournament_manager.tournament_handles.insert(tournament_id, tauri::async_runtime::spawn(update_process.run()));
    open_tournament_manager.open_tournaments.push(tournament_id);
    //.borrow_mut().open_tournaments.push(tournament_id);
    //let mut open_tournament_manager = open_tournament_manager.borrow_mut();

    Ok(())
}

#[tauri::command]
async fn guess_csv_config(path: String) -> Result<CSVReaderConfig, ()> {
    let result = open_tab_app_backend::frontend_queries::query_participant_csv_config_proposal(path).await;

    result.map_err(|_| ())
}

struct OpenTournamentManager {
    open_tournaments: Vec<Uuid>,
    tournament_handles: HashMap<Uuid, tauri::async_runtime::JoinHandle<()>>
}

impl OpenTournamentManager {
    fn new() -> Self {
        Self {
            open_tournaments: vec![],
            tournament_handles: HashMap::new()
        }
    }
}

struct TournamentUpdateProcess {
    tournament_id: Uuid,
    app_handle: AppHandle,
    client: Client,
    sync_frequency: chrono::Duration
}

impl TournamentUpdateProcess {
    async fn run(self) {
        let mut last_sync = None;

        loop {
            if let Some(last_sync) = last_sync {
                let time_passed = chrono::Utc::now().naive_utc() - last_sync;
                if time_passed < self.sync_frequency {
                    tokio::time::sleep((self.sync_frequency - time_passed).to_std().unwrap()).await;
                }
            }
            last_sync = Some(chrono::Utc::now().naive_utc());

            let db = &*self.app_handle.state::<DatabaseConnection>();

            let transaction = db.begin().await.unwrap();
            let target_tournament_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(self.tournament_id)).one(&transaction).await.unwrap();
            if target_tournament_remote.is_none() {
                println!("No remote");
                continue;
            }
            let target_tournament_remote = target_tournament_remote.unwrap();
            transaction.rollback().await.unwrap();
            
            let remote = pull_remote_changes(&target_tournament_remote, &self.client, db, self.app_handle.state::<Mutex<ViewCache>>().inner(), &self.app_handle).await;
            if !remote.is_ok() {
                println!("Error pulling remote changes: {}", remote.err().unwrap());
            }
            else {
            }
            let transaction = db.begin().await.unwrap();
            let target_tournament_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(self.tournament_id)).one(&transaction).await.unwrap();
            if target_tournament_remote.is_none() {
                println!("No remote");
                continue;
            }
            let target_tournament_remote = target_tournament_remote.unwrap();
            transaction.rollback().await.unwrap();

            let result = try_push_changes(&target_tournament_remote, &self.client, db).await;
            if !result.is_ok() {
                println!("Error pushing local changes: {}", result.err().unwrap());
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

#[tauri::command]
async fn set_remote(app: AppHandle, client: State<'_, Client>, db: State<'_, DatabaseConnection>, view_cache: State<'_, Mutex<ViewCache>>, settings_lock: State<'_, RwLock<AppSettings>>, tournament_id: Uuid, remote_url: String) -> Result<(), ()> {
    let settings = settings_lock.read().await;

    let remote = settings.known_remotes.iter().find(|r| r.url == remote_url).map(|r| r.clone()).ok_or(())?;

    let account_id = if let Some(account_id) = remote.account_id {
        Ok(account_id)
    }
    else {
        drop(settings);
        let mut settings = settings_lock.write().await;

        let result = match reqwest::Client::new().post(format!("{}/api/users", remote_url)).json(
            &CreateUserRequest {
                password: "testpassword".to_string(),
            }
        ).send().await {
            Ok(r) => {
                let r : CreateUserResponse = r.json().await.unwrap();
                Ok(r.uuid)
            }
            Err(e) => {
                dbg!("Err with user", e);
                Err(())
            }
        };
        
        let remote = settings.known_remotes.iter_mut().find(|r| r.url == remote_url);

        if let Some(remote) = remote {
            remote.account_id = Some(result?);
        }
        else {
            settings.known_remotes.push(RemoteSettings {
                url: remote_url.clone(),
                name: remote_url.clone(),
                account_id: Some(result?)
            });
        }     

        result
    };

    let account_id = account_id?;
    let transaction = db.begin().await.map_err(|_| ())?;
    let current_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(tournament_id)).one(&transaction).await.unwrap();

    if let Some(current_remote) = current_remote {
        current_remote.delete(&transaction).await.map_err(|_| ())?;
    }

    match client.post(format!("{}/api/tournaments", remote_url)).basic_auth(account_id.to_string(), "testpassword".into()).json(
        &CreateTournamentRequest {
            uuid: tournament_id,
            name: "Test Tournament".to_string(),
        }
    ).send().await {
        Ok(r) => {
            let r : CreateTournamentResponse = r.json().await.unwrap();
            dbg!(&r);
        }
        Err(e) => {
            dbg!("Err", e);
        }
    };

    schema::tournament_remote::ActiveModel {
        uuid: sea_orm::ActiveValue::Set(Uuid::new_v4()),
        tournament_id: sea_orm::ActiveValue::Set(tournament_id),
        url: sea_orm::ActiveValue::Set(remote_url),
        last_known_change: sea_orm::ActiveValue::Set(None),
        last_synced_change: sea_orm::ActiveValue::Set(None)
    }.insert(&transaction).await.map_err(|_| ())?;

    transaction.commit().await.map_err(|_| ())?;

    //FIXME: LoadedTournamentStatusView contains references to remotes, which are not entities
    // and thus not automatically managed. This requries this ugly hack.
    let reloaded_view = Box::new(LoadedTournamentStatusView::load(db.inner(), tournament_id).await.map_err(|_| ())?);

    let change: ChangeNotification = view_cache.lock().await.force_replace(
        View::TournamentStatus { tournament_uuid: tournament_id },
        reloaded_view,
        db.inner()
    ).await.map_err(|_| ())?;

    app.emit_all("views-changed", ChangeNotificationSet {changes: vec![change]}).expect("Event send failed");

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppSettings {
    known_remotes: Vec<RemoteSettings>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteSettings {
    url: String,
    name: String,
    account_id: Option<Uuid>
}

impl AppSettings {
    fn try_read(settings_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let settings_file = File::open(&settings_path)?;

        let settings = serde_json::from_reader(settings_file)?;
        Ok(settings)
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            known_remotes: vec![
                RemoteSettings {
                    url: "http://localhost:3000".to_string(),
                    name: "Local".to_string(),
                    account_id: None
                }
            ]
        }
    }
}

fn main() {
    let db_path = dirs::document_dir().unwrap_or(PathBuf::from(".")).join("open_tab_db.sqlite3");

    let db = block_on(connect_db_to_file(Some(db_path))).unwrap();
    let (_sync_notification_send, _sync_notification_recv) = tauri::async_runtime::channel::<SyncNotification>(100);

    let settings_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("com.juliussteen.open_tab");

    let mut has_user_dir = false;

    if !settings_dir.is_dir() {
        if create_dir(&settings_dir).is_ok() {
            has_user_dir = true;
        }
    }
    else {
        has_user_dir = true;
    }

    let settings_path = settings_dir.join("settings.json");

    if !settings_path.exists() && has_user_dir {
        let settings = AppSettings::default();

        let settings_file = File::create(&settings_path).unwrap();
        serde_json::to_writer(settings_file, &settings).unwrap();
    }

    let settings = AppSettings::try_read(settings_path).unwrap_or_default();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            subscribe_to_view,
            execute_action,
            guess_csv_config,
            evaluate_ballots,
            get_tournament_list,
            open_tournament,
            get_settings,
            set_remote
        ])
        .manage(db)
        .manage(Mutex::new(ViewCache::new()))
        .manage(Mutex::new(OpenTournamentManager::new()))
        .manage(RwLock::new(settings))
        .manage(Client::new())
        .setup(|app| {
            let app_handle = app.handle();
            /*
            let synchronization_function = async move {
                let target_uuid = Uuid::from_u128(1);
                let client = reqwest::Client::new();

                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let db = &*app_handle.state::<DatabaseConnection>();

                    let transaction = db.begin().await.unwrap();
                    let target_tournament_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(target_uuid)).one(&transaction).await.unwrap();
                    if target_tournament_remote.is_none() {
                        println!("No remote");
                        continue;
                    }
                    let target_tournament_remote = target_tournament_remote.unwrap();
                    transaction.rollback().await.unwrap();
                    
                    let remote = pull_remote_changes(&target_tournament_remote, &client, db, app_handle.state::<Mutex<ViewCache>>().inner(), &app_handle).await;
                    if !remote.is_ok() {
                        println!("Error pulling remote changes: {}", remote.err().unwrap());
                    }
                    else {
                    }
                    let transaction = db.begin().await.unwrap();
                    let target_tournament_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(target_uuid)).one(&transaction).await.unwrap();
                    if target_tournament_remote.is_none() {
                        println!("No remote");
                        continue;
                    }
                    let target_tournament_remote = target_tournament_remote.unwrap();
                    transaction.rollback().await.unwrap();

                    let result = try_push_changes(&target_tournament_remote, &client, db).await;
                    if !result.is_ok() {
                        println!("Error pushing local changes: {}", result.err().unwrap());
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            };
             */

            Ok(())  
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
