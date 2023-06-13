use axum::{
    routing::get,
    Router, extract::{MatchedPath, State, FromRef}, http::Request,
};
use db::DatabaseConfig;
use tower_http::{trace::TraceLayer, cors::{CorsLayer, Any}};
use tracing::{info_span, instrument::WithSubscriber};
use tracing_subscriber::prelude::*;
use axum::TypedHeader;
use axum::async_trait;
use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::headers::Authorization;
use axum::headers::authorization::Basic;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use hyper::{http::request::Parts, Method};
use tower::Service; // for `call`
use tower::ServiceExt; // for `oneshot` and `ready`

pub mod auth;
pub mod tournament;
pub mod response;
mod db;
pub mod state;
pub mod ballot;
pub mod participants;
pub mod sync;

use sea_orm::prelude::*;

use state::AppState;
use tracing_subscriber::fmt;

pub async fn app() -> axum::Router<()> {
    app_with_state(AppState::new().await).await
}

pub async fn app_with_state(state: AppState) -> axum::Router<()> {
    let cors = CorsLayer::new()
    // allow `GET` and `POST` when accessing the resource
    .allow_methods([Method::GET, Method::POST])
    // allow requests from any origin
    .allow_origin(Any);

    let app = Router::new().route(
        "/", get(|State(db): State<AppState>| async { "Hello, World!" })
    )
    .nest("/api",
        auth::router().merge(
            tournament::router()
        ).merge(
            ballot::router()
        ).merge(
            participants::router()
        ).merge(
            sync::router()
        )
    )
    .layer(
        TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            // Log the matched route's path (with placeholders not filled in).
            // Use request.uri() or OriginalUri if you want the real path.
            let matched_path = request
                .extensions()
                .get::<MatchedPath>()
                .map(MatchedPath::as_str);

            let uri = request.uri().to_string();

            info_span!(
                "http_request",
                uri = uri,
                method = ?request.method(),
                matched_path,
            )
        })
    ).layer(
        cors
    ).with_state(
        state
    );
    app
}
