use axum::Router;

use crate::state::AppState;



pub(crate) fn router() -> Router<AppState> {
    Router::new()
}