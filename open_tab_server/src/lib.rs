use axum::{
    routing::get,
    Router, extract::{MatchedPath, State}, http::Request,
};

use tower_http::{trace::TraceLayer, cors::{CorsLayer, Any}};
use tracing::info_span;

use axum::http::Method;
 // for `call`
 // for `oneshot` and `ready`

pub mod auth;
pub mod tournament;
pub mod response;
pub mod db;
pub mod state;
pub mod ballot;
pub mod participants;
pub mod sync;
pub mod feedback;
pub mod tab;
pub mod presentation;
pub mod users;
pub mod notify;
pub mod debate;
pub mod patch;

use state::AppState;


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
        "/", get(|State(_db): State<AppState>| async { "Hello, World!" })
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
        ).merge(
            feedback::router()
        ).merge(
            tab::router()
        ).merge(
            presentation::router()
        ).merge(
            users::router()
        ).merge(
            notify::router()
        ).merge(
            debate::router()
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
