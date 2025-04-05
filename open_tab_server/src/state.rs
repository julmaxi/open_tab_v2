use std::sync::Arc;
use axum::extract::FromRef;
use db::DatabaseConfig;
use migration::MigratorTrait;
use tokio::sync::{RwLock};

use crate::{db, cache, notify::ParticipantNotificationManager};
use sea_orm::{prelude::*, Statement};


#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub notifications: Arc<RwLock<ParticipantNotificationManager>>,
    pub cache_manager: Arc<crate::cache::CacheManager>,
}

impl AppState {
    pub async fn new() -> AppState {
        let db = db::set_up_db(
            DatabaseConfig::new(
                "sqlite://./server.sqlite3?mode=rwc".into(),
            )
        ).await.expect("Failed to set up database");
        match &db {
            DatabaseConnection::SqlxSqlitePoolConnection(_) => {
                db.execute(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "PRAGMA foreign_keys = ON;",
                    vec![])
                ).await.expect("Failed to enable foreign keys");
            },
            _ => {}
        }
        migration::Migrator::up(&db, None).await.unwrap();
        AppState {
            db,
            notifications: Arc::new(RwLock::new(ParticipantNotificationManager::new())),
            cache_manager: Arc::new(cache::CacheManager::new((2 as usize).pow(20))),
        }
    }

    pub async fn new_with_db(db: DatabaseConnection) -> AppState {
        match &db {
            DatabaseConnection::SqlxSqlitePoolConnection(_) => {
                db.execute(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "PRAGMA foreign_keys = ON;",
                    vec![])
                ).await.expect("Failed to enable foreign keys");
            },
            _ => {}
        }
        migration::Migrator::up(&db, None).await.unwrap();
        AppState {
            db,
            notifications: Arc::new(RwLock::new(ParticipantNotificationManager::new())),
            cache_manager: Arc::new(cache::CacheManager::new((2 as usize).pow(20))),
        }
    }

    pub async fn new_test_app() -> AppState {
        let db = db::set_up_db(
            DatabaseConfig::new(
                "sqlite::memory:".into(),
            )
        ).await.expect("Failed to set up database");
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "PRAGMA foreign_keys = ON;",
            vec![])
        ).await.expect("Failed to enable foreign keys");
        migration::Migrator::up(&db, None).await.unwrap();
        AppState {
            db,
            notifications: Arc::new(RwLock::new(ParticipantNotificationManager::new())),
            cache_manager: Arc::new(cache::CacheManager::new((2 as usize).pow(20))),
        }
    }
}

impl FromRef<AppState> for DatabaseConnection {
    fn from_ref(app_state: &AppState) -> DatabaseConnection {
        app_state.db.clone()
    }
}

impl FromRef<AppState> for Arc<RwLock<ParticipantNotificationManager>> {
    fn from_ref(app_state: &AppState) -> Arc<RwLock<ParticipantNotificationManager>> {
        app_state.notifications.clone()
    }
}
