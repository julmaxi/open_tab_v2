use axum::{
    routing::get,
    Router, extract::{MatchedPath, State, FromRef}, http::Request,
};
use db::DatabaseConfig;
use migration::MigratorTrait;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::prelude::*;
use axum::TypedHeader;
use axum::async_trait;
use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::headers::Authorization;
use axum::headers::authorization::Basic;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use hyper::http::request::Parts;
use tower::Service; // for `call`
use tower::ServiceExt; // for `oneshot` and `ready`

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
                //"sqlite://./open_tab_v2.sqlite3?mode=rwc".into(),
                "sqlite::memory:".into(),
                "open_tab_v2".into(),
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
