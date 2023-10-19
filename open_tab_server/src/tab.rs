use std::{collections::HashMap, error::Error, hash::Hash};

use axum::{extract::{Path, State}, Json, Router, routing::get};
use hyper::StatusCode;
use itertools::Itertools;
use open_tab_entities::{domain::{entity::LoadEntity, feedback_form::{FeedbackForm, FeedbackFormVisibility, FeedbackSourceRole, FeedbackTargetRole}, round}, prelude::{SpeechRole, TournamentRound}, schema, tab::TabView};
use sea_orm::{DatabaseConnection, TransactionTrait, prelude::*, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::{response::{APIError, handle_error, handle_error_dyn}, auth::{ExtractAuthenticatedUser, AuthenticatedUser, check_release_date}, state::AppState, tournament};

#[derive(Debug, Serialize, Deserialize)]
pub struct TabResponse {
    tab: TabView
}

pub async fn get_current_tab(
    State(db): State<DatabaseConnection>,
    Path(tournament_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
) -> Result<Json<TabResponse>, APIError> {
    if !user.check_is_authorized_in_tournament(&db, tournament_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized for this tournament"));
        return Err(err);
    }
    let tournament_rounds = TournamentRound::get_all_in_tournament(&db, tournament_id).await.map_err(handle_error)?;

    let visible_rounds = tournament_rounds.iter().filter(|round| !round.is_silent).collect_vec();

    let tab = TabView::load_from_tournament_with_rounds(&db, tournament_id, visible_rounds.iter().map(|r| r.uuid).collect_vec()).await?;
    let now = chrono::Utc::now();

    return Ok(
        Json(
            TabResponse {
                tab
            }
        )
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
    .route("/tournament/:tournament_id/tab", get(get_current_tab))
}