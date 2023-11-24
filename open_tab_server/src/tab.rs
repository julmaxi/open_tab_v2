use std::{collections::HashMap};

use axum::{extract::{Path, State}, Json, Router, routing::get};
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{domain::{entity::LoadEntity}, prelude::{TournamentRound}, tab::TabView};
use sea_orm::{DatabaseConnection, prelude::*};
use serde::{Serialize, Deserialize};

use crate::{response::{APIError, handle_error}, auth::{ExtractAuthenticatedUser}, state::AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct TabResponse {
    tab: TabView,
    rounds: Vec<TabRoundInfo>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TabRoundInfo {
    state: TabRoundState,

    tab_index: Option<usize> // We remove silent rounds from the tab, so we need to keep track of the index in the tab
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum TabRoundState {
    Public,
    NotFinished,
    Silent,
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

    let now = chrono::Utc::now().naive_utc();
    let tournament_rounds_with_state = tournament_rounds.iter().map(|r| {
        let state = if r.is_silent {
            TabRoundState::Silent
        } else if r.round_close_time.map_or(false, |t| {
            dbg!(&t);
            t <= now
        }) {
            TabRoundState::Public
        } else {
            TabRoundState::NotFinished
        };

        (r, state)
    }).collect_vec();

    let visible_rounds = tournament_rounds_with_state.iter().enumerate().filter(|round| round.1.1 == TabRoundState::Public).map(|round| (round.0, round.1.0)).collect_vec();
//    let visible_rounds = visible_rounds.into_iter().map(|r| r.1).collect_vec();
    let tab_indices = visible_rounds.iter().enumerate().map(|r| (r.0, r.1.0)).collect::<HashMap<_, _>>();
    let tab = TabView::load_from_tournament_with_rounds(&db, tournament_id, visible_rounds.iter().map(|r| r.1.uuid).collect_vec()).await?;

    let rounds = tournament_rounds_with_state.iter().enumerate().map(|(r_idx, (_r, state))| TabRoundInfo {
        tab_index: tab_indices.get(&r_idx).map(|i| *i),
        state: *state
    }).collect_vec();

    return Ok(
        Json(
            TabResponse {
                tab,
                rounds
            }
        )
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
    .route("/tournament/:tournament_id/tab", get(get_current_tab))
}