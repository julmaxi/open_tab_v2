use std::{str::FromStr, error::Error};

use argon2::Argon2;
use axum::{
    routing::{get, post},
    Router, extract::{MatchedPath, State, Path}, http::Request, Json, headers::authorization::Bearer,
};
use base64::Engine;
use open_tab_entities::{schema::{user_access_key, user}, prelude::Participant};
use rand::{thread_rng, Rng};
use sea_orm::{prelude::*, DatabaseConnection, ActiveValue, IntoActiveModel, TransactionTrait, QuerySelect, Related};
use serde::{Serialize, Deserialize};
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::prelude::*;
use axum::TypedHeader;
use axum::async_trait;
use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::headers::Authorization;
use axum::headers::authorization::Basic;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use hyper::http::request::{Parts, self};
use tower::Service; // for `call`
use tower::ServiceExt;

use crate::{response::{APIError, handle_error, handle_error_dyn}, state::AppState};

use password_hash::{PasswordHash, PasswordVerifier, PasswordHasher, Salt, SaltString};


#[derive(Debug)]
pub struct AuthenticatedUser {
    pub uuid: Uuid,
    pub authorized_only_for_tournament: Option<Uuid>
}

impl AuthenticatedUser {
    pub async fn check_is_authorized_for_tournament_administration<C>(&self, db: &C, tournament_id: Uuid) -> Result<bool, Box<dyn Error>> where C: ConnectionTrait {
        if let Some(authorized_only_for_tournament_id) = self.authorized_only_for_tournament {
            return Ok(authorized_only_for_tournament_id == tournament_id);
        }
        else {
            let user_tournament = open_tab_entities::schema::user_tournament::Entity::find_by_id(
                (self.uuid, tournament_id)
            ).one(db).await?;

            Ok(user_tournament.is_some())
        }
    }

    pub async fn check_is_authorized_as_participant<C>(&self, db: &C, participant_id: Uuid) -> Result<bool, Box<dyn Error>> where C: ConnectionTrait {
        let user_participant_id = open_tab_entities::schema::user_participant::Entity::find().filter(
            open_tab_entities::schema::user_participant::Column::UserId.eq(self.uuid).and(
                open_tab_entities::schema::user_participant::Column::ParticipantId.eq(participant_id)
            )).one(db).await?.map(|u| u.participant_id);   
        
        Ok(user_participant_id == Some(participant_id))
    }

    pub async fn check_is_authorized_as_member_of_team<C>(&self, db: &C, team_id: Uuid) -> Result<bool, Box<dyn Error>> where C: ConnectionTrait {
        let user_participant_id: Option<Uuid> = open_tab_entities::schema::speaker::Entity::find()
        .join(
            sea_orm::JoinType::InnerJoin,
            open_tab_entities::schema::speaker::Entity::belongs_to(open_tab_entities::schema::user_participant::Entity)
            .from(open_tab_entities::schema::speaker::Column::Uuid)
            .to(open_tab_entities::schema::user_participant::Column::ParticipantId)
            .into()
        )
        .filter(
            open_tab_entities::schema::user_participant::Column::UserId.eq(self.uuid).and(
                open_tab_entities::schema::speaker::Column::TeamId.eq(team_id)
            )).one(db).await?.map(|u| u.uuid);   
        
        Ok(user_participant_id.is_some())
    }

    pub async fn participant_id_in_tournament<C>(&self, db: &C, tournament_id: Uuid) -> Result<Option<Uuid>, Box<dyn Error>> where C: ConnectionTrait {
        let user_participant_id = open_tab_entities::schema::user_participant::Entity::find()
        .inner_join(
            open_tab_entities::schema::participant::Entity
        )
        .filter(
            open_tab_entities::schema::user_participant::Column::UserId.eq(self.uuid).and(
                open_tab_entities::schema::participant::Column::TournamentId.eq(tournament_id)
        )).one(db).await?.map(|u| u.participant_id);   
        
        Ok(user_participant_id)
    }

    pub async fn check_is_authorized_in_tournament<C>(&self, db: &C, tournament_id: Uuid) -> Result<bool, Box<dyn Error>> where C: ConnectionTrait {
        Ok(self.participant_id_in_tournament(db, tournament_id).await?.is_some())
    }
}


pub struct ExtractAuthenticatedUser(pub AuthenticatedUser);


#[async_trait]
impl FromRequestParts<AppState> for ExtractAuthenticatedUser
{
    type Rejection = APIError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let basic_header = TypedHeader::<Authorization<Basic>>::from_request_parts(parts, state)
            .await;

        if let Ok(basic_header) = basic_header {
            let decoded = basic_header.0;
            let user_name = decoded.username();
            let password = decoded.password();
    
            let user_uuid = Uuid::from_str(user_name).map_err(|_| (StatusCode::BAD_REQUEST, "User ID is not formatted correcty"))?;
    
            let user = open_tab_entities::schema::user::Entity::find_by_id(
                user_uuid
            ).one(&state.db).await.map_err(handle_error)?;
    
            let user = user.ok_or((StatusCode::UNAUTHORIZED, "User not found or password incorrect"))?;

            let password_hash = PasswordHash::new(&user.password_hash).expect("invalid password hash");
            let algs: &[&dyn PasswordVerifier] = &[&Argon2::default()];

            password_hash.verify_password(algs, password).map_err(|_| (StatusCode::UNAUTHORIZED, "User not found or password incorrect"))?;

            return Ok(ExtractAuthenticatedUser(AuthenticatedUser {
                uuid: user_uuid,
                authorized_only_for_tournament: None
            }))    
        }
        else {
            let TypedHeader(bearer_header) = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
            .await.map_err(|_| {
                (StatusCode::UNAUTHORIZED, "No valid authorization header found")
            })?;
            let key = base64::engine::general_purpose::STANDARD_NO_PAD.decode(&bearer_header.0.token()).unwrap();
            let salt = SaltString::from_b64("bXlzYWx0bXlzYWx0").unwrap();
            let hashed_key = Argon2::default().hash_password(&key, &salt).map_err(
                |_| {
                    (StatusCode::UNAUTHORIZED, "No valid authorization header found")
                }
            )?;

            let key = open_tab_entities::schema::user_access_key::Entity::find_by_id(
                hashed_key.to_string()
            ).one(&state.db).await.map_err(handle_error)?;

            let key = key.ok_or((StatusCode::UNAUTHORIZED, "Bearer token invalid"))?;
            
            return Ok(ExtractAuthenticatedUser(AuthenticatedUser {
                uuid: key.user_id,
                authorized_only_for_tournament: key.tournament_id
            }))    
        }
    }
}



#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub password: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub uuid: Uuid
}


#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GetTokenRequest {
    pub tournament: Option<Uuid>
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GetTokenResponse {
    pub token: String
}


#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RegisterUserResponse {
    pub user_id: Uuid,
    pub participant_id: Uuid,
    pub tournament_id: Uuid,
    pub token: String
}

pub fn hash_password(pwd: String) -> Result<String, Box<dyn Error>> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let pwd = Argon2::default().hash_password(
        pwd.as_bytes(), 
        &salt
    );

    Ok(pwd?.to_string())
}

pub async fn create_user_handler(
    State(db): State<DatabaseConnection>,
    Json(request): Json<CreateUserRequest>
) -> Result<Json<CreateUserResponse>, APIError> {
    let pwd = request.password;
    let new_user_uuid = Uuid::new_v4();
    let pwd = hash_password(pwd).map_err(handle_error_dyn)?;

    let model: open_tab_entities::schema::user::Model = open_tab_entities::schema::user::Model {
        uuid: new_user_uuid,
        password_hash: pwd,
    };

    model.into_active_model().insert(&db).await.map_err(
        handle_error
    )?;

    return Ok(CreateUserResponse {
        uuid: new_user_uuid
    }.into());
}


pub fn create_key(key: &[u8], user_id: Uuid, tournament_id: Option<Uuid>) -> Result<user_access_key::Model, Box<dyn std::error::Error>> {
    let salt = SaltString::from_b64("bXlzYWx0bXlzYWx0").unwrap();
    let hashed_key = Argon2::default().hash_password(key, &salt)?;
    Ok(open_tab_entities::schema::user_access_key::Model {
        key_hash: hashed_key.to_string(),
        user_id,
        tournament_id
    })
}

pub async fn create_token_handler(State(db): State<DatabaseConnection>, ExtractAuthenticatedUser(user): ExtractAuthenticatedUser, Json(request): Json<GetTokenRequest>) -> Result<Json<GetTokenResponse>, APIError> {
    if user.authorized_only_for_tournament.is_some() {
        return Err((StatusCode::UNAUTHORIZED, "Tournament specific tokens can't be used to create new keys").into())
    }

    let key: [u8; 32] = thread_rng().gen::<[u8; 32]>();

    let token = create_key(&key, user.uuid, request.tournament).map_err(handle_error_dyn)?;
    token.into_active_model().insert(&db).await.map_err(handle_error)?;

    return Ok(
        GetTokenResponse {
            token: base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key)
        }.into()
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterParticipantRequest {
    pub secret: String
}

pub async fn register_user_handler(
    State(db): State<DatabaseConnection>,
    Json(RegisterParticipantRequest{secret}): Json<RegisterParticipantRequest>
) -> Result<Json<RegisterUserResponse>, APIError> {
    let (participant_id, submitted_key) = Participant::decode_registration_key(secret)?;

    let participant = open_tab_entities::schema::participant::Entity::find_by_id(participant_id)
        .one(&db).await.map_err(handle_error)?.ok_or(APIError::from((StatusCode::BAD_REQUEST, "Participant not found or key invalid")))?;

    let db = db.begin().await.map_err(handle_error)?;

    match participant.registration_key {
        None => Err((StatusCode::BAD_REQUEST, "Participant can not be claimed").into()),
        Some(registration_key) => {
            if registration_key == submitted_key {
                let existing_user = open_tab_entities::schema::user_participant::Entity::find().filter(
                    open_tab_entities::schema::user_participant::Column::ParticipantId.eq(participant_id)
                ).one(&db).await.map_err(handle_error)?;

                if let Some(existing_user) = existing_user {
                    let key: [u8; 32] = thread_rng().gen::<[u8; 32]>();
                    let token = create_key(&key, existing_user.user_id, None).map_err(handle_error_dyn)?;
                    token.into_active_model().insert(&db).await.map_err(handle_error)?;
                    db.commit().await.map_err(handle_error)?;
                    Ok(
                        RegisterUserResponse {
                            user_id: existing_user.user_id,
                            participant_id: participant_id,
                            tournament_id: participant.tournament_id,
                            token: base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key)
                        }.into()
                    )
                }
                else {
                    let new_user_id = Uuid::new_v4();
                    let new_user = open_tab_entities::schema::user::Model {
                        uuid: new_user_id,
                        password_hash: "".to_string()
                    };

                    new_user.into_active_model().insert(&db).await.map_err(handle_error)?;
                    let key: [u8; 32] = thread_rng().gen::<[u8; 32]>();
                    let user_key = create_key(&key, new_user_id, None).map_err(handle_error_dyn)?;
                    user_key.into_active_model().insert(&db).await.map_err(handle_error)?;

                    let user_participant = open_tab_entities::schema::user_participant::Model {
                        user_id: new_user_id,
                        participant_id
                    };
                    user_participant.into_active_model().insert(&db).await.map_err(handle_error)?;
                    
                    db.commit().await.map_err(handle_error)?;
                    Ok(
                        RegisterUserResponse {
                            user_id: new_user_id,
                            participant_id,
                            tournament_id: participant.tournament_id,
                            token: base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key)
                        }.into()
                    )
            }
            }
            else {
                db.rollback().await.map_err(handle_error)?;
                Err((StatusCode::BAD_REQUEST, "Incorrect key or participant id").into())
            }
        }
    }
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/users", post(create_user_handler))
        .route("/tokens", post(create_token_handler))
        .route("/register", post(register_user_handler))
}

pub fn check_release_date(current_time: chrono::NaiveDateTime, check_time: Option<chrono::NaiveDateTime>) -> bool {
    if let Some(check_time) = check_time {
        current_time > check_time
    } else {
        false
    }
}