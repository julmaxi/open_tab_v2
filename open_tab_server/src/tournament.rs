use axum::extract::State;
use axum::{Json, Router, routing::post};
use axum::body::Body;
use base64::Engine;
use hyper::StatusCode;
use open_tab_entities::{EntityGroup, EntityGroupTrait};
use rand::{thread_rng, Rng};
use sea_orm::{DatabaseConnection, IntoActiveModel, ActiveModelTrait};
use sea_orm::prelude::Uuid;
use serde::{Serialize, Deserialize};

use crate::auth::{create_key, ExtractAuthenticatedUser};
use crate::response::{APIError, handle_error_dyn, handle_error};
use crate::state::AppState;


#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CreateTournamentRequest {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CreateTournamentResponse {
    pub access_key: Option<String>,
    pub uuid: Uuid,
}


pub async fn create_tournament_handler(State(db) : State<DatabaseConnection>, ExtractAuthenticatedUser(user) : ExtractAuthenticatedUser, Json(request): Json<CreateTournamentRequest>) -> Result<Json<CreateTournamentResponse>, APIError> {
    let mut changes = EntityGroup::new();
    let uuid = request.uuid;
    let tournament = open_tab_entities::domain::tournament::Tournament {
        uuid,
        annoucements_password: Some("password".into())
    };
    changes.add(open_tab_entities::Entity::Tournament(tournament));

    // TODO: Prevent overriding tournament
    //changes.save_all_and_log_for_tournament(&db, uuid).await.map_err(handle_error_dyn)?;
    changes.save_all(&db).await.map_err(handle_error_dyn)?;
    let key = thread_rng().gen::<[u8; 32]>();
    let token = create_key(&key, user.uuid, Some(uuid)).map_err(handle_error_dyn)?;
    token.into_active_model().insert(&db).await.map_err(handle_error)?;

    return Ok(
        Json(
            CreateTournamentResponse {
                uuid,
                access_key: Some(base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key)),
            }
        )
    )
}


pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/tournaments", post(create_tournament_handler))
}