use axum::{extract::{Path, State}, routing::get, Json, Router};
use sea_orm::{prelude::Uuid, DatabaseConnection, EntityTrait};
use serde::Serialize;

use crate::{auth::ExtractAuthenticatedUser, response::{handle_error, APIError}, state::AppState};

#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    identifier: String,
}

pub async fn get_user_info(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
) -> Result<Json<UserInfo>, APIError> {
    let user = open_tab_entities::schema::user::Entity::find_by_id(user.uuid).one(&db).await.map_err(handle_error)?;

    return Ok(Json(UserInfo {
        identifier: user.unwrap().user_email.unwrap_or("Anonymous User".to_string()),
    }));
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/user", get(get_user_info))
}