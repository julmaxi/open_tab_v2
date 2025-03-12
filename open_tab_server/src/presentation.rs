use axum::{extract::{State, Path}, Json, Router, routing::{post, get}};
use chrono::Duration;
use axum::http::StatusCode;

use open_tab_entities::{derived_models::{DrawPresentationInfo, LoadDrawError}, domain::{self, entity::LoadEntity}, schema, EntityGroup};
use sea_orm::{prelude::Uuid, DatabaseConnection, EntityTrait, TransactionTrait};
use serde::{Serialize, Deserialize};

use crate::{auth::ExtractAuthenticatedUser, response::APIError, state::AppState};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DrawPresentationInfoWithTime {
    #[serde(flatten)]
    presentation_info: DrawPresentationInfo,
    debate_start_time: chrono::NaiveDateTime,
}

async fn get_draw_presentation(
    State(db): State<DatabaseConnection>,
    Path(round_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
) -> Result<Json<DrawPresentationInfoWithTime>, APIError> {
    let round = schema::tournament_round::Entity::find_by_id(round_id)
        .one(&db)
        .await?;
    if round.is_none() {
        return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Round not found"))
    }
    let round = round.unwrap();
    if !user.check_is_authorized_for_tournament_administration(&db, round.tournament_id).await? {
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not authorized for this tournament"))
    }
    let presentation_info = DrawPresentationInfo::load_for_round(&db, round_id).await;

    match presentation_info {
        Ok(presentation_info) => {
            let debate_start_time = round.debate_start_time.unwrap_or_else(|| chrono::Utc::now().naive_utc() + Duration::minutes(15));
            Ok(Json(
                DrawPresentationInfoWithTime {
                    presentation_info,
                    debate_start_time,
                }
            ))
        },
        Err(LoadDrawError::NotFound) => {
            Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Round not found"))
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
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(round_id): Path<Uuid>,
) -> Result<Json<ReleaseMotionResponse>, APIError> {
    let db = db.begin().await?;
    let round = domain::round::TournamentRound::try_get(&db, round_id).await?;

    if !round.is_some() {
        db.rollback().await?;
        return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Round not found"))
    }

    let mut round = round.unwrap();
    let tournament_id = round.tournament_id;

    if !user.check_is_authorized_for_tournament_administration(&db, tournament_id).await? {
        db.rollback().await?;
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not authorized for this tournament"))
    }

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
    db.commit().await?;

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
