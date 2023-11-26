use axum::extract::FromRef;
use db::DatabaseConfig;
use migration::MigratorTrait;

use crate::db;
use sea_orm::{prelude::*, Statement};


#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection
}

impl AppState {
    pub async fn new() -> AppState {
        let db = db::set_up_db(
            DatabaseConfig::new(
                "sqlite://./server.sqlite3?mode=rwc".into(),
            )
        ).await.expect("Failed to set up database");
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "PRAGMA foreign_keys = ON;",
            vec![])
        ).await.expect("Failed to enable foreign keys");
        migration::Migrator::up(&db, None).await.unwrap();
        AppState {
            db
        }
    }

    pub async fn new_with_db(db: DatabaseConnection) -> AppState {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "PRAGMA foreign_keys = ON;",
            vec![])
        ).await.expect("Failed to enable foreign keys");
        migration::Migrator::up(&db, None).await.unwrap();
        AppState {
            db
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
            db
        }
    }
}

impl FromRef<AppState> for DatabaseConnection {
    fn from_ref(app_state: &AppState) -> DatabaseConnection {
        app_state.db.clone()
    }
}
