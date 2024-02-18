// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, error::Error, fmt::{Display, Formatter, Debug}, iter::zip, path::{PathBuf, Path}, fs::File};

use identity::IdentityProvider;
use migration::MigratorTrait;
use open_tab_entities::{EntityGroup, domain::{ballot::{SpeechRole, BallotParseError}, entity::LoadEntity, self}, schema::{self}, utilities::BatchLoadError, EntityType, derived_models::{DrawPresentationInfo, RegistrationInfo}, tab::{TabView, BreakRelevantTabView}};
use open_tab_reports::{TemplateContext, make_open_office_ballots, template::{make_open_office_tab, OptionallyBreakRelevantTab, make_open_office_presentation, make_pdf_registration_items}};
use open_tab_server::{sync::{SyncRequestResponse, SyncRequest, FatLog, reconcile_changes, ReconciliationOutcome}, tournament::CreateTournamentRequest, auth::{CreateUserRequest, CreateUserResponse, GetTokenResponse, GetTokenRequest, CreateUserRequestError}, response::APIErrorResponse};
//use open_tab_server::TournamentChanges;
use reqwest::Client;
use sea_orm::{prelude::*, Statement, Database, DatabaseTransaction, TransactionTrait, ActiveValue};
use tauri::{async_runtime::block_on, State, AppHandle, Manager, api::dialog::MessageDialogBuilder};
use open_tab_entities::prelude::*;
use itertools::Itertools;
use serde::{Serialize, Deserialize};

use open_tab_app_backend::{View, draw_view::DrawBallot, LoadedView, Action, import::CSVReaderConfig, draw::evaluation::{DrawIssue, DrawEvaluator}, tournament_status_view::LoadedTournamentStatusView, feedback::FormTemplate};

use thiserror::Error;
use tokio::{sync::Mutex, sync::RwLock};

use std::sync::Arc;

mod tournament_creation;
mod identity;


#[allow(dead_code)]
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

    pub async fn get_view_string<C>(&mut self, view: View, db: &C) -> Result<String, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let loaded_view = self.get_view(view, db).await?;
        let view_str = loaded_view.view_string().await?;

        Ok(view_str)
    }

    pub async fn force_replace<C>(&mut self, view: View, new_values: Box<dyn LoadedView>, db: &C) -> Result<ChangeNotification, anyhow::Error> where C: sea_orm::ConnectionTrait {
        self.cached_views.insert(view.clone(), new_values);
        //FIXME: Hacky
        let view_string = self.get_view_string(view.clone(), db).await?;
        let value : serde_json::Value = serde_json::from_str(&view_string)?;
        Ok(ChangeNotification {
            view: view,
            updated_paths: HashMap::from([(".".into(), value)]),
        })
    }

    pub async fn update_and_get_changes(&mut self, db: &DatabaseTransaction, changes: &EntityGroup) -> Result<Vec<ChangeNotification>, anyhow::Error> {
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

    pub async fn get_view<C>(&mut self, view: View, db: &C) -> Result<&Box<dyn LoadedView>, anyhow::Error> where C: sea_orm::ConnectionTrait {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionResponse {
    success: bool,
    message: Option<String>,
}

async fn execute_action_impl(action: Action, db: &DatabaseConnection, view_cache: &mut ViewCache) -> Result<Vec<ChangeNotification>, anyhow::Error> {
    let transaction = db.begin().await?;
    let changes: EntityGroup = action.execute(&transaction).await?;
    let deleted_tournaments = changes.get_all_deletion_tournaments(&transaction).await?;
    changes.save_all(&transaction).await?;
    let tournament = changes.get_all_tournaments(&transaction).await?.into_iter().chain(deleted_tournaments.into_iter()).filter_map(|s| s).next();
    if let Some(tournament) = tournament {
        changes.save_log_with_tournament_id(&transaction, tournament).await?;

        transaction.commit().await?;
        let transaction = db.begin().await?;
    
        let notifications = view_cache.update_and_get_changes(&transaction, &changes).await?;
        transaction.commit().await?;    
        Ok(notifications)
    }
    else {
        anyhow::bail!("No tournament found in changes");
    }
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
            name: model.name,
            uuid: model.uuid
        }
    }
}

#[tauri::command]
async fn get_tournament_list(db: State<'_, DatabaseConnection>) -> Result<Vec<TournamentListEntry>, ()> {
    let tournaments = schema::tournament::Entity::find().all(db.inner()).await.map_err(|_| ())?;

    Ok(tournaments.into_iter().map(TournamentListEntry::from).collect())
}


fn handle_error<E>(e: E) where E: Debug {
    dbg!(&e);
}


#[derive(Debug)]
enum SyncError {
    ReqwestError(reqwest::Error),
    DatabaseError(sea_orm::DbErr),
    Other(String),
    TournamentDoesNotExist,
    NotAuthorized,
    SyncRejection,
    LogsOutOfSync
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

impl From<anyhow::Error> for SyncError {
    fn from(err: anyhow::Error) -> Self {
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
            SyncError::Other(err) => write!(f, "Other error: {}", err),
            SyncError::NotAuthorized => write!(f, "Not authorized"),
            SyncError::TournamentDoesNotExist => write!(f, "Tournament does not exist"),
            SyncError::SyncRejection => write!(f, "Sync rejected"),
            SyncError::LogsOutOfSync => write!(f, "Logs out of sync"),
        }
    }
}

impl std::error::Error for SyncError {}

async fn auto_accept_ballots<C>(changes: &EntityGroup, db: &C) -> Result<Option<EntityGroup>, SyncError> where C: sea_orm::ConnectionTrait {
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
            new_changes.delete(EntityType::Ballot, old_ballot.uuid);
        }
    };

    Ok(Some(new_changes))
}

async fn pull_remote_changes<C>(
    target_tournament_remote: &schema::tournament_remote::Model,
    client: &Client,
    db: &C,
    api_key: &String,
    view_cache: &Mutex<ViewCache>, 
    app_handle: &AppHandle
) -> Result<Option<EntityGroup>, SyncError> where C: sea_orm::ConnectionTrait + TransactionTrait {
    let mut remote_url = format!("{}/api/tournament/{}/log", target_tournament_remote.url, target_tournament_remote.tournament_id);

    if let Some(last_common_ancestor) = target_tournament_remote.last_synced_change {
        remote_url = format!("{}?since={}", remote_url, last_common_ancestor);
    }

    let response = client.get(remote_url).bearer_auth(api_key).send().await?;
    
    if response.status() == 403 || response.status() == 401 {
        return Err(SyncError::NotAuthorized);
    }
    if response.status() == 404 {
        return Err(SyncError::TournamentDoesNotExist);
    }

    if response.status() == 500 {
        let error_response = response.json::<APIErrorResponse<String>>().await?;
        //FIXME: This would be nicer with typed error codes
        if error_response.message == "Since is not a valid log entry" {
            return Err(SyncError::LogsOutOfSync)
        }
        return Err(SyncError::Other(format!("Server error: {}", error_response.message)));
    }
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

                if let Some(entity_group) = entity_group {
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
                }

                return Ok(None);
            }
            ReconciliationOutcome::Reject => {
                transaction.rollback().await?;
                return Err(SyncError::Other("Reconciliation failed".to_string()));
            }
            ReconciliationOutcome::InvalidTournament => {
                transaction.rollback().await?;
                return Err(SyncError::Other("Reconciliation failed: Invalid tournament".to_string()));
            }
        }

    }

    Ok(None)
}

async fn try_push_changes<C>(target_tournament_remote: &schema::tournament_remote::Model, client: &Client, api_key: &String, db: &C) -> Result<(), SyncError> where C: sea_orm::ConnectionTrait + TransactionTrait {
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

    let response = client.post(remote_url).bearer_auth(
        api_key
    ).json(&
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
    
                let update = schema::tournament_remote::ActiveModel {                
                    uuid: ActiveValue::Unchanged(target_tournament_remote.uuid),
                    last_synced_change: ActiveValue::Set(Some(new_last_common_ancestor)),
                    ..Default::default()
                };
                update.update(&transaction).await?;
    
                transaction.commit().await?;
            }
            open_tab_server::sync::APIReconciliationOutcome::Reject | open_tab_server::sync::APIReconciliationOutcome::InvalidTournament => {
                return Err(SyncError::SyncRejection);
            }
        };    
    }
    else if response.status() == 403 || response.status() == 401 {
        return Err(SyncError::NotAuthorized);
    }
    else if response.status() == 404 {
        return Err(SyncError::TournamentDoesNotExist);
    }
    else {
        return Err(SyncError::Other(format!("Unexpected response status: {}", response.status())));   
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
            non_aligned_speakers: b.non_aligned_speakers.iter().filter_map(|s| s.as_ref()).map(|s| r.non_aligned_issues.get(&s.uuid).map(|i| i.clone()).unwrap_or(Vec::new()).into_iter().filter(|i| i.target.uuid() == target_uuid).collect_vec()).collect_vec(),
            adjudicators: b.adjudicators.iter().map(|s| r.adjudicator_issues.get(&s.adjudicator.uuid).map(|i| i.clone()).unwrap_or(Vec::new()).into_iter().filter(|i| i.target.uuid() == target_uuid).collect_vec()).collect_vec(),
        }
    ).collect())
}

#[tauri::command]
async fn get_settings(settings: State<'_, RwLock<AppSettings>>) -> Result<AppSettings, ()> {
    Ok(settings.inner().read().await.clone())
}

#[tauri::command]
async fn open_tournament(
    handle: AppHandle,
    _client: State<'_, Client>,
    open_tournament_manager: State<'_, Arc<Mutex<OpenTournamentManager>>>,
    identity_provider: State<'_, Arc<IdentityProvider>>,
    _db: State<'_, DatabaseConnection>,
    tournament_id: Uuid
) -> Result<(), ()> {
    let _tournament_window = tauri::WindowBuilder::new(
        &handle,
        &format!("tournament:{}", tournament_id.to_string()), /* the unique window label */
        tauri::WindowUrl::App("index.html".into())
    ).title(&tournament_id.to_string()).build().map_err(|_| ())?;
      //tournament_window.emit("select_tournament", tournament_id).map_err(|_| ())?;

    handle.get_window("main").map(|h|
        h.close()
    );
 
    OpenTournamentManager::open_tournament(open_tournament_manager.inner().clone(), &handle, tournament_id, identity_provider.inner().clone()).await.map_err(|_| ())?;

    /*let update_process = TournamentUpdateProcess {
        tournament_id: tournament_id,
        app_handle: handle.clone(),
        client: client.inner().clone(),
        sync_frequency: chrono::Duration::seconds(10),
        update_msg_sender: open_tournament_manager.inner().lock().await.update_msg_sender.clone()
    };

    let mut open_tournament_manager = open_tournament_manager.inner().lock().await;
    let open_tournament_manager = open_tournament_manager.borrow_mut();

    open_tournament_manager.process_states.insert(tournament_id, ConnectivityStatus::Disconnect { timestamp: chrono::Utc::now().naive_utc() });
    open_tournament_manager.tournament_handles.insert(tournament_id, tauri::async_runtime::spawn(update_process.run()));
    open_tournament_manager.open_tournaments.push(tournament_id);*/

    Ok(())
}

#[tauri::command]
async fn guess_csv_config(path: String) -> Result<CSVReaderConfig, ()> {
    let result: Result<CSVReaderConfig, anyhow::Error> = open_tab_app_backend::frontend_queries::query_participant_csv_config_proposal(path).await;

    result.map_err(|_| ())
}

struct OpenTournamentManager {
    tournament_processes: HashMap<Uuid, ProcessInfo>,
    update_msg_sender: tokio::sync::mpsc::Sender<ConnectivityStatusMessage>,
}

#[derive(Debug)]
struct ProcessInfo {
    #[allow(dead_code)]
    join_handle: tauri::async_runtime::JoinHandle<Result<(), TournamentUpdateError>>,
    process_state: ConnectivityStatus,
}


impl OpenTournamentManager {
    fn new(update_msg_sender : tokio::sync::mpsc::Sender<ConnectivityStatusMessage>  ) -> Self {
        Self {
            tournament_processes: HashMap::new(),
            update_msg_sender
        }
    }

    async fn run_event_manager(
        info: Arc<Mutex<OpenTournamentManager>>,
        mut msg_queue: tokio::sync::mpsc::Receiver<ConnectivityStatusMessage>,
        app_handle: AppHandle
    ) {
        loop {
            if let Some(msg) = msg_queue.recv().await {
                info.lock().await.tournament_processes.get_mut(&msg.tournament_id).map(|p| p.process_state = msg.status.clone());
                app_handle.emit_all("connectivity-update", msg).expect("Event send failed");
            }
        }
    }
    
    async fn open_tournament(
        info: Arc<Mutex<OpenTournamentManager>>,
        app_handle: &AppHandle,
        id: Uuid,
        identity_provider: Arc<IdentityProvider>,
    ) -> Result<(), TournamentUpdateError> {
        let mut info = info.lock().await;
        let curr_process = info.tournament_processes.get(
            &id
        );

        if curr_process.is_some() {
            return Ok(());
        }

        let settings = app_handle.state::<RwLock<AppSettings>>().inner().read().await;
        let client = app_handle.state::<Client>().inner().clone();

        let tournament_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(id)).one(&*app_handle.state::<DatabaseConnection>()).await.unwrap();

        if let Some(tournament_remote) = tournament_remote {
            let _remote = settings.known_remotes.iter().find(|r| r.url == tournament_remote.url).unwrap();
            let process = TournamentUpdateProcess {
                tournament_id: id,
                app_handle: app_handle.clone(),
                client,
                sync_frequency: chrono::Duration::seconds(5),
                update_msg_sender: info.update_msg_sender.clone(),
                identity_provider
            };
    
            let join_handle = tauri::async_runtime::spawn(process.run());
        
            info.tournament_processes.insert(id, ProcessInfo { join_handle, process_state: ConnectivityStatus::Disconnect { timestamp: chrono::Utc::now().naive_utc() }});
        }

        Ok(())
    }
}

#[tauri::command]
async fn get_tournament_connectivity_status(open_tournament_manager: State<'_, Arc<Mutex<OpenTournamentManager>>>, tournament_id: Uuid) -> Result<Option<ConnectivityStatus>, ()> {
    let open_tournament_manager = open_tournament_manager.inner().lock().await;
    Ok(open_tournament_manager.tournament_processes.get(&tournament_id).map(|t| t.process_state.clone()))
}

struct TournamentUpdateProcess {
    tournament_id: Uuid,
    app_handle: AppHandle,
    client: Client,
    sync_frequency: chrono::Duration,
    update_msg_sender: tokio::sync::mpsc::Sender<ConnectivityStatusMessage>,
    identity_provider: Arc<IdentityProvider>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectivityStatusMessage {
    tournament_id: Uuid,
    #[serde(flatten)]
    status: ConnectivityStatus
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
enum ConnectivityStatus {
    Alive {timestamp: DateTime},
    Error {message: String},
    Connect {timestamp: DateTime},
    Disconnect {timestamp: DateTime},
    PasswordRequired {timestamp: DateTime}
}


#[derive(Debug, thiserror::Error)]
enum TournamentUpdateError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error("No remote")]
    NoRemote,
    #[error("Other error: {0}")]
    Other(String)
}


impl TournamentUpdateProcess {
    async fn run(self) -> Result<(), TournamentUpdateError> {
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

            let transaction = db.begin().await?;
            let target_tournament_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(self.tournament_id)).one(&transaction).await.unwrap();
            if target_tournament_remote.is_none() {
                println!("No remote");

                self.update_msg_sender.send(ConnectivityStatusMessage {
                    tournament_id: self.tournament_id,
                    status: ConnectivityStatus::Disconnect { timestamp: chrono::Utc::now().naive_utc()}
                }).await.expect("Error sending connectivity update");

                break Err(TournamentUpdateError::NoRemote);
            }
            let target_tournament_remote = target_tournament_remote.unwrap();
            transaction.rollback().await.unwrap();

            let _settings : State<'_, RwLock<AppSettings>> = self.app_handle.state();
            let api_key = self.identity_provider.try_get_key(&target_tournament_remote.url).await;
            
            let api_key = if let Some(api_key) = api_key {
                api_key
            } else {
                self.update_msg_sender.send(ConnectivityStatusMessage {
                    tournament_id: self.tournament_id,
                    status: ConnectivityStatus::PasswordRequired { timestamp: chrono::Utc::now().naive_utc() }
                }).await.expect("Error sending connectivity update");

                println!("No key");

                self.identity_provider.get_key_blocking(&target_tournament_remote.url).await.map_err(|e| TournamentUpdateError::Other(e.to_string()))?
            };

            if target_tournament_remote.created_at.is_none() {
                let tournament = schema::tournament::Entity::find_by_id(self.tournament_id).one(&*self.app_handle.state::<DatabaseConnection>()).await.map_err(|e| TournamentUpdateError::DatabaseError(e))?.ok_or(TournamentUpdateError::Other("Could not find tournament".into()))?;
                let result = self.client.post(format!("{}/api/tournaments", target_tournament_remote.url)).bearer_auth(api_key.clone()).json(
                    &CreateTournamentRequest {
                        uuid: self.tournament_id,
                        name: tournament.name,
                    }
                ).send().await;

                if let Err(result) = result {
                    println!("Error creating tournament: {}", result);
                    self.update_msg_sender.send(ConnectivityStatusMessage {
                        tournament_id: self.tournament_id,
                        status: ConnectivityStatus::Error { message: result.to_string() }
                    }).await.expect("Error sending connectivity update");
                    continue;
                    //break Err(TournamentUpdateError::ReqwestError(result));
                }

                let transaction = db.begin().await?;
                let update = schema::tournament_remote::ActiveModel {                
                    uuid: ActiveValue::Unchanged(target_tournament_remote.uuid),
                    created_at: ActiveValue::Set(Some(chrono::Utc::now().naive_utc())),
                    last_known_change: ActiveValue::Set(None),
                    last_synced_change: ActiveValue::Set(None),
                    ..Default::default()
                };
                update.update(&transaction).await?;
                transaction.commit().await?;
            }

            
            let remote = pull_remote_changes(&target_tournament_remote, &self.client, db, &api_key, self.app_handle.state::<Mutex<ViewCache>>().inner(), &self.app_handle).await;
            if !remote.is_ok() {
                let err = remote.err().unwrap();
                match &err {
                    SyncError::NotAuthorized => {
                        self.update_msg_sender.send(ConnectivityStatusMessage {
                            tournament_id: self.tournament_id,
                            status: ConnectivityStatus::PasswordRequired { timestamp: chrono::Utc::now().naive_utc() }
                        }).await.expect("Error sending connectivity update");
                        continue;
                    },
                    SyncError::TournamentDoesNotExist => {
                        let transaction = db.begin().await?;
                        let update = schema::tournament_remote::ActiveModel {                
                            uuid: ActiveValue::Unchanged(target_tournament_remote.uuid),
                            created_at: ActiveValue::Set(None),
                            ..Default::default()
                        };
                        update.update(&transaction).await?;
                        transaction.commit().await?;        
                    },
                    SyncError::LogsOutOfSync => {
                        let transaction = db.begin().await?;
                        let update = schema::tournament_remote::ActiveModel {                
                            uuid: ActiveValue::Unchanged(target_tournament_remote.uuid),
                            last_synced_change: ActiveValue::Set(None),
                            last_known_change: ActiveValue::Set(None),
                            ..Default::default()
                        };
                        update.update(&transaction).await?;
                        transaction.commit().await?;
                    }
                    _ => {}
                }
                println!("Error pulling remote changes: {}", err);
                self.update_msg_sender.send(ConnectivityStatusMessage {
                    tournament_id: self.tournament_id,
                    status: ConnectivityStatus::Error { message: err.to_string() }
                }).await.expect("Error sending connectivity update");
                continue;
            }
            let transaction = db.begin().await.unwrap();
            let target_tournament_remote = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(self.tournament_id)).one(&transaction).await.unwrap();
            if target_tournament_remote.is_none() {
                println!("No remote");
                continue;
            }
            let target_tournament_remote = target_tournament_remote.unwrap();
            transaction.rollback().await.unwrap();

            let result = try_push_changes(&target_tournament_remote, &self.client, &api_key, db).await;
            if !result.is_ok() {
                let err = result.err().unwrap();
                match &err {
                    SyncError::NotAuthorized => {
                        self.update_msg_sender.send(ConnectivityStatusMessage {
                            tournament_id: self.tournament_id,
                            status: ConnectivityStatus::PasswordRequired { timestamp: chrono::Utc::now().naive_utc() }
                        }).await.expect("Error sending connectivity update");
                        continue;
                    },
                    SyncError::TournamentDoesNotExist => {
                        let transaction = db.begin().await?;
                        let update = schema::tournament_remote::ActiveModel {                
                            uuid: ActiveValue::Unchanged(target_tournament_remote.uuid),
                            created_at: ActiveValue::Set(None),
                            ..Default::default()
                        };
                        update.update(&transaction).await?;
                        transaction.commit().await?;        
                    },
                    _ => {}
                }
                println!("Error pushing local changes: {}", err);
                continue;
            }
            
            self.update_msg_sender.send(ConnectivityStatusMessage {
                tournament_id: self.tournament_id,
                status: ConnectivityStatus::Alive { timestamp: chrono::Utc::now().naive_utc() }
            }).await.expect("Error sending connectivity update");
        }
    }
}

#[tauri::command]
async fn set_remote(
    app: AppHandle, _client: State<'_, Client>,
    db: State<'_, DatabaseConnection>,
    open_tournament_manager: State<'_, Arc<Mutex<OpenTournamentManager>>>,
    view_cache: State<'_, Mutex<ViewCache>>,
    settings_lock: State<'_, RwLock<AppSettings>>,
    identity_provider: State<'_, Arc<IdentityProvider>>,
    tournament_id: Uuid,
    remote_url: String) -> Result<(), ()> {
    let settings = settings_lock.read().await;

    let _remote = settings.known_remotes.iter().find(|r| r.url == remote_url).map(|r| r.clone()).ok_or(())?;

    let transaction = db.begin().await.map_err(|_| ())?;

    // We might have accidentally set multiple remotes for the same tournament
    let prev_remotes = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(tournament_id)).all(&transaction).await.map_err(|_| ())?;
    if prev_remotes.len() > 0 {
        schema::tournament_remote::Entity::delete_many().filter(schema::tournament_remote::Column::TournamentId.eq(tournament_id)).exec(&transaction).await.map_err(|_| ())?;
    }

    schema::tournament_remote::ActiveModel {
        uuid: sea_orm::ActiveValue::Set(Uuid::new_v4()),
        tournament_id: sea_orm::ActiveValue::Set(tournament_id),
        url: sea_orm::ActiveValue::Set(remote_url),
        last_known_change: sea_orm::ActiveValue::Set(None),
        last_synced_change: sea_orm::ActiveValue::Set(None),
        created_at: sea_orm::ActiveValue::Set(None)
    }.insert(&transaction).await.map_err(|_| ())?;

    transaction.commit().await.map_err(|_| ())?;

    OpenTournamentManager::open_tournament(open_tournament_manager.inner().clone(), &app, tournament_id, identity_provider.inner().clone()).await.map_err(|_| ())?;

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
    known_remotes: Vec<RemoteSettings>,
    known_api_keys: HashMap<String, String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoteSettings {
    url: String,
    name: String,
    account_name: Option<String>,
}

impl AppSettings {
    fn settings_path() -> PathBuf {
        let settings_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("com.juliussteen.open-tab");
        let settings_path = settings_dir.join("settings.json");
        settings_path
    }

    fn try_read() -> Result<Self, anyhow::Error> {
        let settings_path = Self::settings_path();
        let settings_file = File::open(&settings_path)?;

        let settings = serde_json::from_reader(settings_file)?;
        Ok(settings)
    }

    fn write(&self) -> Result<(), anyhow::Error> {
        let path = Self::settings_path();
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        let settings_file = File::create(&path)?;

        serde_json::to_writer(settings_file, &self)?;
        Ok(())
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            known_remotes: vec![
                RemoteSettings {
                    url: "https://api.debateresult.com".to_string(),
                    name: "Default".to_string(),
                    account_name: None,
                },
                RemoteSettings {
                    url: "http://localhost:3000".to_string(),
                    name: "Local".to_string(),
                    account_name: None,
                }
            ],
            known_api_keys: HashMap::new()
        }
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "error")]
enum LoginError {
    #[error("Incorrect password")]
    IncorrectPassword,
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("User Exists")]
    UserExists,
    #[error("Password too short")]
    PasswordTooShort,
}

#[tauri::command]
async fn login_to_remote(
    db: State<'_, DatabaseConnection>,
    open_tournament_manager: State<'_, Arc<Mutex<OpenTournamentManager>>>,
    client: State<'_, Client>,
    settings: State<'_, RwLock<AppSettings>>,
    identity_provider: State<'_, Arc<IdentityProvider>>,
    remote_url: String,
    user_name: String,
    password: String,
) -> Result<bool, LoginError> {

    run_login(
        &*db,
        &client,
        remote_url.clone(),
        user_name.clone(),
        password.clone(),
        open_tournament_manager.clone(),
        settings.clone(),
        identity_provider.inner().clone()
    ).await.map_err(|e| {
        e
    })?;

    Ok(true)
}

async fn run_login(
    db: &DatabaseConnection,
    client: &Client,
    remote_url: String,
    user_name: String,
    password: String,
    _open_tournament_manager: State<'_, Arc<Mutex<OpenTournamentManager>>>,
    settings: State<'_, RwLock<AppSettings>>,
    identity_provider: Arc<IdentityProvider>
) -> Result<(), LoginError> {
    let response = client.post(
        format!("{}/api/tokens", remote_url)
    ).json(
        &GetTokenRequest {
            tournament: None
        }
    ).basic_auth(format!("mail#{}", user_name), Some(password)).send().await.map_err(|e| LoginError::NetworkError(e.to_string()))?;

    if response.status() == 401 {
        return Err(LoginError::IncorrectPassword);
    }

    //GetTokenRequest
    let response = response.json::<GetTokenResponse>().await.map_err(|e| LoginError::NetworkError(e.to_string()))?;

    let token = response.token;

    settings.write().await.known_api_keys.insert(remote_url.clone(), token.clone());

    let r = settings.write().await.write();
    if r.is_err() {
        dbg!(r.unwrap_err());
    }

    let _all_remotes = open_tab_entities::schema::tournament_remote::Entity::find().filter(
        open_tab_entities::schema::tournament_remote::Column::Url.eq(remote_url.clone())
    ).all(&*db).await.unwrap();

    identity_provider.set_key(remote_url, token).await;

    //let mut open_tournament_manager = open_tournament_manager.inner().lock().await;
    //let update_msg_sender = open_tournament_manager.update_msg_sender.clone();

    /*
    for remote in all_remotes {
        let process_info = open_tournament_manager.tournament_processes.get(&remote.tournament_id);
        let is_finished = if let Some(process_info) = process_info {
            let is_finished = process_info.join_handle.inner().is_finished();
            is_finished
        } else {
            true
        };

        if is_finished {
            let (config_msg_sender, config_msg_receiver) = tokio::sync::mpsc::channel::<ProcessSettingsUpdate>(1);
            let new_tournament_process = TournamentUpdateProcess {
                tournament_id: remote.tournament_id,
                app_handle: app_handle.clone(),
                client: client.clone(),
                sync_frequency: chrono::Duration::seconds(10),
                update_msg_sender: update_msg_sender.clone(),
                identity_provider: identity_provider.clone()
            };

            let join_handle = tauri::async_runtime::spawn(new_tournament_process.run());
            open_tournament_manager.tournament_processes.insert(remote.tournament_id, ProcessInfo { join_handle, process_state: ConnectivityStatus::Disconnect { timestamp: chrono::Utc::now().naive_utc() }, config_msg_sender });
        }
        continue;
        /*if open_tournament_manager.open_tournaments.contains(&remote.tournament_id) {
            settings.write().await.tournament_api_keys.entry(remote_url.clone()).or_insert(HashMap::new()).insert(remote.tournament_id, token.clone());

            let new_tournament_update = TournamentUpdateProcess {
                tournament_id: remote.tournament_id,
                app_handle: app_handle.clone(),
                client: client.clone(),
                sync_frequency: chrono::Duration::seconds(10),
                update_msg_sender: update_msg_sender.clone()
            };
            tauri::async_runtime::spawn(new_tournament_update.run());
        }*/
    } */

    Ok(())
}


#[tauri::command]
async fn create_user_account_for_remote(
    _app_handle: AppHandle,
    db: State<'_, DatabaseConnection>,
    client: State<'_, Client>,
    settings: State<'_, RwLock<AppSettings>>,
    identity_provider: State<'_, Arc<IdentityProvider>>,
    open_tournament_manager: State<'_, Arc<Mutex<OpenTournamentManager>>>,
    remote_url: String,
    user_name: String,
    password: String,
) -> Result<bool, LoginError> {
    let url: String = format!("{}/api/users", remote_url);

    let response = client.post(
        url
    ).json(
        &CreateUserRequest {
            password: password.clone(),
            user_email: Some(user_name.clone()),
        }
    ).send().await.map_err(|e| LoginError::NetworkError(e.to_string()))?;

    if response.status() == 200 {
        let _response = response.json::<CreateUserResponse>().await.map_err(|e| LoginError::NetworkError(e.to_string()))?;

        let mut settings_lock = settings.write().await;
        let remote = settings_lock.known_remotes.iter_mut().find(
            |r| r.url == remote_url
        );
    
        if let Some(remote) = remote {
            remote.account_name = Some(user_name.clone());
        }
        else {
            settings_lock.known_remotes.push(RemoteSettings {
                url: remote_url.clone(),
                name: remote_url.clone(),
                account_name: Some(user_name.clone()),
            });
        }
        settings_lock.known_api_keys.insert(remote_url.clone(), password.clone());
    
        //TODO: Log this somewhere
        let _ = settings_lock.write().map_err(|_| ());
    
        drop(settings_lock);
    
        run_login(&db, &client, remote_url, user_name, password, open_tournament_manager, settings, identity_provider.inner().clone()).await?;
    
        Ok(true)    
    }
    else {
        let response = response.json::<APIErrorResponse<CreateUserRequestError>>().await.map_err(|e| LoginError::NetworkError(e.to_string()))?;

        match response.message {
            CreateUserRequestError::UserExists => Err(LoginError::UserExists),
            CreateUserRequestError::PasswordTooShort => Err(LoginError::PasswordTooShort),
            CreateUserRequestError::Other(s) => Err(LoginError::NetworkError(s.into()))
        }
    }
}

#[tauri::command]
async fn save_round_files(db: State<'_, DatabaseConnection>, template_context: State<'_, TemplateContext>, round_id: Uuid, dir_path: String) -> Result<(), ()> {
    let presentation = DrawPresentationInfo::load_for_round(db.inner(), round_id).await.map_err(handle_error)?;

    let file = File::create(Path::new(&dir_path).join(format!("ballots_r{}.odg", presentation.round_index + 1))).map_err(handle_error)?;
    make_open_office_ballots(&template_context, file, &presentation).map_err(handle_error)?;
    let presentation_file = File::create(Path::new(&dir_path).join(format!("presentation_r{}.odp", presentation.round_index + 1))).map_err(handle_error)?;
    make_open_office_presentation(&template_context, presentation_file, &presentation).map_err(handle_error)?;

    Ok(())
}


#[tauri::command]
async fn save_participant_qr_codes(db: State<'_, DatabaseConnection>, template_context: State<'_, TemplateContext>, tournament_id: Uuid, out_path: String) -> Result<(), ()> {
    let registration_info = RegistrationInfo::load_from_tournament(db.inner(), tournament_id).await.map_err(handle_error)?;

    let file = File::create(out_path).map_err(handle_error)?;
    make_pdf_registration_items(&template_context.inner(), file, registration_info).map_err(handle_error)?;

    Ok(())
}


#[tauri::command]
async fn save_tab(db: State<'_, DatabaseConnection>, template_context: State<'_, TemplateContext>, tournament_id: Uuid, node_id: Option<Uuid>, path: String) -> Result<(), ()> {
    let tab_view = match node_id {
        Some(node_id) => {
            let tab_view = BreakRelevantTabView::load_from_node_with_anonymity(db.inner(), node_id, true).await.map_err(handle_error)?;
            OptionallyBreakRelevantTab::BreakRelevantTab(tab_view)
        },
        None => {
            let tab_view = TabView::load_from_tournament_with_anonymity(db.inner(), tournament_id, true).await.map_err(handle_error)?;
            OptionallyBreakRelevantTab::Tab(tab_view)
        }
    };

    let tournament = domain::tournament::Tournament::get(db.inner(), tournament_id).await.map_err(handle_error)?;

    let file = File::create(path).map_err(handle_error)?;
    make_open_office_tab(&template_context, file, tab_view, tournament.name).map_err(handle_error)?;

    Ok(())
}

#[tauri::command]
async fn create_tournament(app: AppHandle, db: State<'_, DatabaseConnection>, config: tournament_creation::TournamentCreationConfig) -> Result<open_tab_entities::domain::tournament::Tournament, ()> {
    let mut tournament = open_tab_entities::domain::tournament::Tournament::new();
    let (all_nodes, all_edges) = config.get_tournament_graph(tournament.uuid);
    tournament.name = config.name;
    let mut changes = EntityGroup::new();

    if config.use_default_feedback_system {
        let template_path = app.path_resolver().resolve_resource("resources/default_feedback_form.yml");
        if let Some(template_path) = template_path {
            let template_file = File::open(template_path).map_err(handle_error)?;
            let result = FormTemplate::from_reader(template_file).map_err(handle_error)?;

            let (forms, questions) = result.into_forms_and_questions_for_tournament(
                tournament.uuid
            ).map_err(handle_error)?;
    
            for form in forms {
                changes.add(
                    Entity::FeedbackForm(form)
                );
            }
    
            for question in questions {
                changes.add(
                    Entity::FeedbackQuestion(question)
                );
            }
        }
        else {
            println!("Could not read default feedback form. Continuing without.")
        }
    }


    changes.add(Entity::Tournament(tournament.clone()));
    all_nodes.into_iter().for_each(
        |n| changes.add(Entity::TournamentPlanNode(n))
    );
    all_edges.into_iter().for_each(
        |e| changes.add(Entity::TournamentPlanEdge(e))
    );

    changes.save_all_and_log_for_tournament(db.inner(), tournament.uuid).await.map_err(handle_error)?;

    Ok(tournament)
}

struct DownloadProgress {
    pub total: Option<u64>,
    pub downloaded: u64,
}

impl DownloadProgress {
    fn new() -> Self {
        Self {
            total: None,
            downloaded: 0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadProgressMessage {
    pub progress: Option<f32>,
    pub error: Option<String>
}

async fn show_error(error: String) {
    MessageDialogBuilder::new("Update Error", format!("There has been an error during the update process: {}", error)).buttons(
        tauri::api::dialog::MessageDialogButtons::Ok 
    ).kind(
        tauri::api::dialog::MessageDialogKind::Error
    ).show(|_| {});
}

fn main() {
    let db_path: PathBuf = dirs::document_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or(PathBuf::from("."))).join("open_tab_db.sqlite3");

    let db = block_on(connect_db_to_file(Some(db_path))).unwrap();

    let settings = AppSettings::try_read().unwrap_or_default();

    let (send, recv) = tokio::sync::mpsc::channel(100);
    let open_tournaments_manager = Arc::new(Mutex::new(OpenTournamentManager::new(
        send
    )));

    let identity_provider = Arc::new(IdentityProvider::new_with_keys(settings.known_api_keys.clone()));


    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            subscribe_to_view,
            execute_action,
            guess_csv_config,
            evaluate_ballots,
            get_tournament_list,
            open_tournament,
            get_settings,
            set_remote,
            get_tournament_connectivity_status,
            login_to_remote,
            create_user_account_for_remote,
            save_round_files,
            create_tournament,
            save_tab,
            save_participant_qr_codes
        ])
        .manage(db)
        .manage(Mutex::new(ViewCache::new()))
        .manage(std::sync::Mutex::new(DownloadProgress::new()))
        .manage(open_tournaments_manager)
        .manage(RwLock::new(settings))
        .manage(Client::new())
        .manage(identity_provider)
        .setup(|app: &mut tauri::App| {
            let template_path = app.path_resolver().resolve_resource("../../open_tab_reports/templates").expect("Could not resolve template path");
            let template_context = TemplateContext::new(template_path.to_string_lossy().into_owned()).expect("Could not create template context");
            app.manage(template_context);

            let open_tournaments_manager = app.state::<Arc<Mutex<OpenTournamentManager>>>().inner().clone();

            tauri::async_runtime::spawn(
                OpenTournamentManager::run_event_manager(
                    open_tournaments_manager,
                    recv,
                    app.handle().clone()
                )
            );

            Ok(())  
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::Updater(updater_event) => {
          match updater_event {
            tauri::UpdaterEvent::UpdateAvailable { body, date, version } => {
                if app_handle.get_window("update").is_none() {
                    let updater_window = tauri::WindowBuilder::new(
                        app_handle,
                        "update", /* the unique window label */
                        tauri::WindowUrl::App("index.html".into())
                    ).title("Update Progress").build().map_err(|_| ()).unwrap();
                    updater_window.show().unwrap();    
                }
            }
            tauri::UpdaterEvent::Pending => {
            }
            tauri::UpdaterEvent::DownloadProgress { chunk_length, content_length } => {
                if app_handle.get_window("update").is_none() {
                    let updater_window = tauri::WindowBuilder::new(
                        app_handle,
                        "update", /* the unique window label */
                        tauri::WindowUrl::App("index.html".into())
                    ).title("Update Progress").build().map_err(|_| ()).unwrap();
                    updater_window.show().unwrap();    
                }
                let mut progress = app_handle.state::<std::sync::Mutex<DownloadProgress>>().inner().lock().expect("Could not lock progress");
                progress.total = content_length;
                progress.downloaded += chunk_length as u64;

                let msg = DownloadProgressMessage {
                    progress: progress.total.map(|t| progress.downloaded as f32 / t as f32),
                    error: None
                };

                app_handle.emit_to("update", "update-download-progress", msg).expect("Could not send progress");
            }
            tauri::UpdaterEvent::Downloaded => {
                app_handle.get_window("update").map(|h|
                    h.close()
                );
            }
            tauri::UpdaterEvent::Updated => {
            }
            tauri::UpdaterEvent::AlreadyUpToDate => {
            }
            tauri::UpdaterEvent::Error(error) => {
                tauri::async_runtime::spawn(show_error(error));
            }
            _ => (),
          }
        }
        _ => {}});
}
