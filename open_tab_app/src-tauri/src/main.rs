// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, error::Error, hash::Hash, fmt::{Display, Formatter}, sync::Mutex};

use migration::{MigratorTrait, async_trait::async_trait};
use open_tab_entities::{EntityGroups, domain::{tournament::Tournament, ballot::SpeechRole}, schema::{adjudicator, self}};
use sea_orm::{prelude::*, Statement, Database, DatabaseTransaction, TransactionTrait};
use tauri::{async_runtime::block_on, State, App, AppHandle, Manager};
use open_tab_entities::prelude::*;
use itertools::{Itertools, izip};
use serde::{Serialize, Deserialize};

use open_tab_app_backend::{View, draw_view::{DrawDebate, DrawBallot, DrawView, LoadedView}, Action, mock::make_mock_tournament_with_options};


// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


async fn connect_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _ = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    let mock_data = make_mock_tournament_with_options(true);
    let tournament_uuid = mock_data.tournaments[0].uuid.clone();
    mock_data.save_all_with_options(&db, true).await.unwrap();
    mock_data.save_log_with_tournament_id(&db, tournament_uuid).await.unwrap();

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
    pub changes: HashMap<String, serde_json::Value>
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

    pub async fn update_and_get_changes(&mut self, db: &DatabaseTransaction, changes: &EntityGroups) -> Result<Vec<ChangeNotification>, Box<dyn Error>> {
        let mut out = vec![];
        for (view, loaded_view) in self.cached_views.iter_mut() {
            let changes = loaded_view.update_and_get_changes(db, changes).await?;
            if let Some(changes) = changes {
                out.push(ChangeNotification {
                    view: view.clone(),
                    changes
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
fn subscribe_to_view(view: View, db: State<DatabaseConnection>, view_cache: State<Mutex<ViewCache>>) -> SubscriptionResponse {
    // TODO: Handle and report load errors
    let mut view_cache = view_cache.lock().unwrap();
    let view_text = block_on(
        view_cache.get_view_string(view.clone(), db.inner())
    );
    //let view_text = block_on(view.load_json(db.inner()));

    view_text.map(|text| {
        SubscriptionResponse::Success(text)
    }).unwrap_or_else(|err| {
        SubscriptionResponse::Error(err.to_string())
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionResponse {
    success: bool,
    message: Option<String>,
}

async fn execute_action_impl(action: Action, db: &DatabaseConnection, view_cache: &mut ViewCache) -> Result<Vec<ChangeNotification>, Box<dyn Error>> {
    let transaction = db.begin().await?;
    let changes = action.execute(&transaction).await?;
    let tournament = changes.get_all_tournaments(&transaction).await?.into_iter().next().unwrap().unwrap();
    changes.save_all_and_log_for_tournament(&transaction, tournament).await?;

    let notifications = view_cache.update_and_get_changes(&transaction, &changes).await?;
    Ok(notifications)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangeNotificationSet {
    changes: Vec<ChangeNotification>
}


#[tauri::command]
fn execute_action(app: AppHandle, action: Action, db: State<DatabaseConnection>, view_cache: State<Mutex<ViewCache>>) -> ActionResponse {
    let mut view_cache = view_cache.lock().unwrap();
    let result = block_on(execute_action_impl(action, db.inner(), &mut *view_cache));

    match result {
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
    }
}

fn main() {
    let db = block_on(connect_db()).unwrap();
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![subscribe_to_view, execute_action])
        .manage(db)
        .manage(Mutex::new(ViewCache::new()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
