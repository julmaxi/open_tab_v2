use sea_orm::prelude::*;

use open_tab_entities::{derived_models::draw_presentation::{
    DebatePresentationInfo, DrawPresentationInfo
}, domain::round::check_release_date, schema};
use axum::{
    extract::{Path, State}, http::StatusCode, response::Json, routing::get, Router
};
use serde::Serialize;

use crate::{
    auth::MaybeExtractAuthenticatedUser,
    response::APIError,
    state::AppState
};

#[derive(Debug, Serialize)]
struct RoundDrawInfo {
    round_name: String,
    debates: Vec<DebatePresentationInfo>,
}

async fn get_draw_handler(
    State(db) : State<DatabaseConnection>,
    Path(round_id): Path<Uuid>,
    MaybeExtractAuthenticatedUser(user) : MaybeExtractAuthenticatedUser
) -> Result<Json<RoundDrawInfo>, APIError> {

    let round = schema::tournament_round::Entity::find_by_id(round_id)
        .find_also_related(schema::tournament::Entity)
        .one(&db)
        .await
        ?;

    if !round.is_some() {
        return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Round not found"));
    }

    let (round, tournament) = round.unwrap();
    let tournament = tournament.unwrap(); // Guaranteed by db constraints
    
    let mut is_authorized = false;
    if let Some(user) = user {
        is_authorized = user.check_is_authorized_in_tournament(&db, tournament.uuid).await?;
    }

    if !is_authorized {
        let published_tournament = schema::published_tournament::Entity::find()
            .filter(schema::published_tournament::Column::TournamentId.eq(tournament.uuid))
            .one(&db)
            .await
            ?;

        if published_tournament.is_none() {
            return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Tournament not found"));
        }

        if !published_tournament.unwrap().show_draws {
            return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "Draw not available"));
        }
    }

    let now = chrono::Utc::now().naive_utc();
    if !check_release_date(now, round.draw_release_time) {
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "Draw has not been released yet"));
    }

    let presentation_info = DrawPresentationInfo::load_for_round(&db, round_id)
        .await
        ?;
    
    Ok(Json(
        RoundDrawInfo {
            round_name: format!("Round {}", round.index),
            debates: presentation_info.debates,
        }
    ))
}


pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/rounds/:round_id/draw", get(get_draw_handler))
}