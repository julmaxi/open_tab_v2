

use axum::{extract::{State, Path}, Json, Router, routing::{post, get}};
use chrono::Duration;
use axum::http::StatusCode;

use open_tab_entities::{domain::{self, entity::LoadEntity}, EntityGroup, derived_models::{DrawPresentationInfo, LoadDrawError}};
use sea_orm::{prelude::Uuid, DatabaseConnection, TransactionTrait};
use serde::{Serialize, Deserialize};

use crate::{response::{APIError, handle_error}, state::AppState};



async fn get_draw_presentation(
    State(db): State<DatabaseConnection>,
    Path(round_id): Path<Uuid>,
) -> Result<Json<DrawPresentationInfo>, APIError> {
    let presentation_info = DrawPresentationInfo::load_for_round(&db, round_id).await;

    match presentation_info {
        Ok(presentation_info) => {
            Ok(Json(
                presentation_info
            ))
        },
        Err(LoadDrawError::NotFound) => {
            Err(APIError::from((StatusCode::NOT_FOUND, "Round not found")))
        },
        Err(LoadDrawError::DbError(err)) => {
            Err(APIError::from(anyhow::Error::from(err)))
        },
        Err(LoadDrawError::ParticpantParseError(err)) => {
            Err(APIError::from(anyhow::Error::from(err)))
        },
        Err(LoadDrawError::Other(err)) => {
            Err(APIError::from(err))
        },
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
struct ReleaseMotionResponse {
    debate_start_time: chrono::NaiveDateTime
}

async fn set_motion_release(
    State(db): State<DatabaseConnection>,
    Path(round_id): Path<Uuid>,
) -> Result<Json<ReleaseMotionResponse>, APIError> {
    let db = db.begin().await.map_err(handle_error)?;
    let round = domain::round::TournamentRound::try_get(&db, round_id).await?;

    if !round.is_some() {
        return Err(APIError::from((StatusCode::NOT_FOUND, "Round not found")))
    }

    let mut round = round.unwrap();
    let tournament_id = round.tournament_id;

    let now = chrono::Utc::now().naive_utc();

    let draw_release_time = round.draw_release_time.unwrap_or(now);
    round.draw_release_time = Some(draw_release_time);

    let motion_release_time = round.team_motion_release_time.unwrap_or(now);
    round.team_motion_release_time = Some(motion_release_time);

    let debate_start_time = round.debate_start_time.unwrap_or(now + Duration::minutes(15));
    round.debate_start_time = Some(debate_start_time);

    let full_motion_release_time = round.full_motion_release_time.unwrap_or(now + Duration::minutes(20));
    round.full_motion_release_time = Some(full_motion_release_time);

    let mut entity_group = EntityGroup::new(
        tournament_id
    );

    entity_group.add(
        open_tab_entities::Entity::TournamentRound(round)
    );

    entity_group.save_all_and_log(&db).await?;
    db.commit().await.map_err(handle_error)?;

    Ok(Json(
        ReleaseMotionResponse {
            debate_start_time
        }
    ))
}


pub fn router() -> Router<AppState> {
    Router::new()
    .route("/draw/:round_id", get(get_draw_presentation))
    .route("/draw/:round_id/release-motion", post(set_motion_release))
}
