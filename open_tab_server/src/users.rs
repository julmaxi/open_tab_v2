use axum::{extract::{Path, State}, routing::get, Json, Router};
use sea_orm::{prelude::*, DatabaseConnection, EntityTrait, QueryOrder};
use serde::Serialize;

use crate::{auth::ExtractAuthenticatedUser, response::APIError, state::AppState};

#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    identifier: String,
}

pub async fn get_user_info(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
) -> Result<Json<UserInfo>, APIError> {
    let user = open_tab_entities::schema::user::Entity::find_by_id(user.uuid).one(&db).await?;

    return Ok(Json(UserInfo {
        identifier: user.unwrap().user_email.unwrap_or("Anonymous User".to_string()),
    }));
}

#[derive(Debug, Clone, Serialize)]
pub struct UserTournamentInfo {
    participant_id: Option<Uuid>,
}

pub async fn get_user_tournament_info(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(tournament_id): Path<Uuid>,
) -> Result<Json<UserTournamentInfo>, APIError> {
    let user_participants = open_tab_entities::schema::user_participant::Entity::find()
        .inner_join(open_tab_entities::schema::participant::Entity)
        .filter(open_tab_entities::schema::user_participant::Column::UserId.eq(user.uuid))
        .filter(open_tab_entities::schema::participant::Column::TournamentId.eq(tournament_id))
        //Ensure consistent results in the unlikely event of multiple participants
        //linked to the same user.
        .order_by_asc(open_tab_entities::schema::participant::Column::Uuid)
        .all(&db)
        .await
        ?;

    let participant_id = user_participants.first().map(|up| up.participant_id);
    return Ok(Json(UserTournamentInfo {
        participant_id,
    }));
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/user", get(get_user_info))
        .route("/user/tournament/:tournament_id", get(get_user_tournament_info))
}