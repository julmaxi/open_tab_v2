

use axum::{extract::{Path, State}, Json, Router, routing::get};
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{prelude::TournamentRound, tab::TabView};
use sea_orm::{DatabaseConnection, prelude::*};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use crate::{auth::MaybeExtractAuthenticatedUser, cache::CacheManager, response::APIError, state::AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct TabResponse {
    tab: TabView,
}



pub async fn get_current_tab(
    State(state): State<AppState>,
    Path(tournament_id): Path<Uuid>,
    MaybeExtractAuthenticatedUser(user): MaybeExtractAuthenticatedUser,
) -> Result<Json<TabResponse>, APIError> {
    let db = state.db.clone();
    let cache_manager = state.cache_manager.clone();
    let published_tournament = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament_id))
        .one(&db)
        .await?;
    
    let allow_unchecked_access = published_tournament.map(|t| t.show_tab).unwrap_or(false);
    if !allow_unchecked_access {
        if let Some(user) = user {
            if !user.check_is_authorized_in_tournament(&db, tournament_id).await? {
                let err = APIError::new_with_status(StatusCode::FORBIDDEN, "You are not authorized for this tournament");
                return Err(err);
            }
        }
        else {
            let err = APIError::new_with_status(StatusCode::UNAUTHORIZED, "You must be logged in to access this tournament");
            return Err(err);
        }
    }
    let tournament_rounds = TournamentRound::get_all_in_tournament(&db, tournament_id).await?;

    let now = chrono::Utc::now().naive_utc();
    let visible_rounds = tournament_rounds.iter().filter(|r| {
        if r.is_silent && !r.silent_round_results_release_time.map_or(false, |t| {
            t <= now
        }) {
            false
        }
        else if r.round_close_time.map_or(false, |t| {
            t <= now
        }) {
            true
        }
        else {
            false
        }
    }).sorted_by_key(|r| r.index).collect_vec();

    //leshow_anonymityt tab = TabView::load_from_tournament_with_rounds_with_anonymity(&db, tournament_id, visible_rounds.iter().map(|r| r.uuid).collect_vec(), true).await?;
    let tab = cache_manager.get_tab(tournament_id, visible_rounds.iter().map(|r| r.uuid).collect_vec(), true, &db).await?;

    return Ok(
        Json(
            TabResponse {
                tab,
            }
        )
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
    .route("/tournament/:tournament_id/tab", get(get_current_tab))
}