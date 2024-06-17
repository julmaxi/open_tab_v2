use std::str::FromStr;

use argon2::Argon2;
use axum::routing::get;
use axum::{extract::State, headers::authorization::Bearer, routing::post, Json, Router};
use base64::Engine;
use chrono::Duration;
use open_tab_entities::schema;
use open_tab_entities::{prelude::Participant, schema::user_access_key};
use rand::{thread_rng, Rng};
use sea_orm::{prelude::*, DatabaseConnection, IntoActiveModel, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};

use axum::async_trait;
use axum::TypedHeader;

use axum::extract::{FromRequestParts, Path};
use axum::headers::authorization::Basic;
use axum::headers::Authorization;
use axum::http::StatusCode;

use axum::http::request::Parts;
// for `call`

use crate::response::{TypedAPIError, handle_typed_error};
use crate::{
    response::{handle_error, handle_error_dyn, APIError},
    state::AppState,
};

use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

#[derive(Debug)]
pub struct AuthenticatedUser {
    pub uuid: Uuid,
    pub authorized_only_for_tournament: Option<Uuid>,
    pub is_password_authorized: bool,
    pub is_access_only: bool
}

impl AuthenticatedUser {
    pub async fn check_is_authorized_for_tournament_administration<C>(
        &self,
        db: &C,
        tournament_id: Uuid,
    ) -> Result<bool, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        if let Some(authorized_only_for_tournament_id) = self.authorized_only_for_tournament {
            return Ok(authorized_only_for_tournament_id == tournament_id);
        } else {
            let user_tournament = open_tab_entities::schema::user_tournament::Entity::find_by_id((
                self.uuid,
                tournament_id,
            ))
            .one(db)
            .await?;

            Ok(user_tournament.is_some())
        }
    }

    pub async fn check_is_authorized_as_participant<C>(
        &self,
        db: &C,
        participant_id: Uuid,
    ) -> Result<bool, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        let user_participant_id = open_tab_entities::schema::user_participant::Entity::find()
            .filter(
                open_tab_entities::schema::user_participant::Column::UserId
                    .eq(self.uuid)
                    .and(
                        open_tab_entities::schema::user_participant::Column::ParticipantId
                            .eq(participant_id),
                    ),
            )
            .one(db)
            .await?
            .map(|u| u.participant_id);

        Ok(user_participant_id == Some(participant_id))
    }

    pub async fn check_is_authorized_as_member_of_team<C>(
        &self,
        db: &C,
        team_id: Uuid,
    ) -> Result<bool, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        let user_participant_id: Option<Uuid> = open_tab_entities::schema::speaker::Entity::find()
            .join(
                sea_orm::JoinType::InnerJoin,
                open_tab_entities::schema::speaker::Entity::belongs_to(
                    open_tab_entities::schema::user_participant::Entity,
                )
                .from(open_tab_entities::schema::speaker::Column::Uuid)
                .to(open_tab_entities::schema::user_participant::Column::ParticipantId)
                .into(),
            )
            .filter(
                open_tab_entities::schema::user_participant::Column::UserId
                    .eq(self.uuid)
                    .and(open_tab_entities::schema::speaker::Column::TeamId.eq(team_id)),
            )
            .one(db)
            .await?
            .map(|u| u.uuid);

        Ok(user_participant_id.is_some())
    }

    pub async fn participant_id_in_tournament<C>(
        &self,
        db: &C,
        tournament_id: Uuid,
    ) -> Result<Option<Uuid>, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        let user_participant_id = open_tab_entities::schema::user_participant::Entity::find()
            .inner_join(open_tab_entities::schema::participant::Entity)
            .filter(
                open_tab_entities::schema::user_participant::Column::UserId
                    .eq(self.uuid)
                    .and(
                        open_tab_entities::schema::participant::Column::TournamentId
                            .eq(tournament_id),
                    ),
            )
            .one(db)
            .await?
            .map(|u| u.participant_id);

        Ok(user_participant_id)
    }

    pub async fn check_is_authorized_in_tournament<C>(
        &self,
        db: &C,
        tournament_id: Uuid,
    ) -> Result<bool, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        Ok(self
            .participant_id_in_tournament(db, tournament_id)
            .await?
            .is_some())
    }
}

pub struct ExtractAuthenticatedUser(pub AuthenticatedUser);

pub struct MaybeExtractAuthenticatedUser(pub Option<AuthenticatedUser>);

#[async_trait]
impl FromRequestParts<AppState> for ExtractAuthenticatedUser {
    type Rejection = APIError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let basic_header =
            TypedHeader::<Authorization<Basic>>::from_request_parts(parts, state).await;

        if let Ok(basic_header) = basic_header {
            let decoded = basic_header.0;
            let user_name = decoded.username();
            let password = decoded.password();

            let user = if user_name.starts_with("mail#") {
                open_tab_entities::schema::user::Entity::find()
                    .filter(
                        open_tab_entities::schema::user::Column::UserEmail
                            .eq(user_name.trim_start_matches("mail#")),
                    )
                    .one(&state.db)
                    .await
                    .map_err(handle_error)?
            } else {
                let user_uuid = Uuid::from_str(user_name)
                    .map_err(|_| (StatusCode::BAD_REQUEST, "User ID is not formatted correcty"))?;
                open_tab_entities::schema::user::Entity::find_by_id(user_uuid)
                    .one(&state.db)
                    .await
                    .map_err(handle_error)?
            };

            let user = user.ok_or((
                StatusCode::UNAUTHORIZED,
                "User not found or password incorrect",
            ))?;

            let password_hash =
                PasswordHash::new(&user.password_hash).expect("invalid password hash");
            let algs: &[&dyn PasswordVerifier] = &[&Argon2::default()];

            password_hash.verify_password(algs, password).map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "User not found or password incorrect",
                )
            })?;

            return Ok(ExtractAuthenticatedUser(AuthenticatedUser {
                uuid: user.uuid,
                authorized_only_for_tournament: None,
                is_password_authorized: true,
                is_access_only: false
            }));
        } else {
            let TypedHeader(bearer_header) =
                TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                    .await
                    .map_err(|_| {
                        (
                            StatusCode::UNAUTHORIZED,
                            "No valid authorization header found",
                        )
                    })?;
            let key = base64::engine::general_purpose::STANDARD_NO_PAD
                .decode(&bearer_header.0.token())
                .unwrap();
            let salt = SaltString::from_b64("bXlzYWx0bXlzYWx0").unwrap();
            let hashed_key = Argon2::default().hash_password(&key, &salt).map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "No valid authorization header found",
                )
            })?;

            let key = open_tab_entities::schema::user_access_key::Entity::find_by_id(
                hashed_key.to_string(),
            )
            .one(&state.db)
            .await
            .map_err(handle_error)?;

            let key = key.ok_or((StatusCode::UNAUTHORIZED, "Bearer token invalid"))?;

            return Ok(ExtractAuthenticatedUser(AuthenticatedUser {
                uuid: key.user_id,
                authorized_only_for_tournament: key.tournament_id,
                is_password_authorized: false,
                is_access_only: key.is_access_only
            }));
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for MaybeExtractAuthenticatedUser {
    type Rejection = APIError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let authenticated_user = ExtractAuthenticatedUser::from_request_parts(parts, state).await;
        if let Ok(authenticated_user) = authenticated_user {
            Ok(MaybeExtractAuthenticatedUser(Some(authenticated_user.0)))
        } else {
            Ok(MaybeExtractAuthenticatedUser(None))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub password: String,
    pub user_email: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub uuid: Uuid,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GetTokenRequest {
    #[serde(default)]
    pub tournament: Option<Uuid>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GetTokenResponse {
    pub token: String,
    pub expires: Option<i64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RegisterUserResponse {
    pub user_id: Uuid,
    pub participant_id: Uuid,
    pub tournament_id: Uuid,
    pub token: String,
}

pub fn hash_password(pwd: String) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let pwd = Argon2::default().hash_password(pwd.as_bytes(), &salt);

    Ok(pwd?.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateUserRequestError {
    UserExists,
    PasswordTooShort,
    Other(String)
}

impl From<String> for CreateUserRequestError {
    fn from(s: String) -> Self {
        CreateUserRequestError::Other(s)
    }
}

pub async fn create_user_handler(
    State(db): State<DatabaseConnection>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<CreateUserResponse>, TypedAPIError<CreateUserRequestError>> {
    let pwd = request.password;

    if pwd.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            CreateUserRequestError::PasswordTooShort,
        )
            .into());
    }

    if let Some(user_email) = &request.user_email {
        let existing_user = open_tab_entities::schema::user::Entity::find()
            .filter(open_tab_entities::schema::user::Column::UserEmail.eq(user_email))
            .one(&db)
            .await
            .map_err(handle_typed_error)?;

        if existing_user.is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                CreateUserRequestError::UserExists,
            )
                .into());
        }
    }

    let new_user_uuid = Uuid::new_v4();
    let pwd = hash_password(pwd)?;
    let model: open_tab_entities::schema::user::Model = open_tab_entities::schema::user::Model {
        uuid: new_user_uuid,
        password_hash: pwd,
        user_email: request.user_email,
    };

    model
        .into_active_model()
        .insert(&db)
        .await
        .map_err(handle_typed_error)?;

    return Ok(CreateUserResponse {
        uuid: new_user_uuid,
    }
    .into());
}

pub fn create_key(
    key: &[u8],
    user_id: Uuid,
    tournament_id: Option<Uuid>,
    validity_duration: Option<chrono::Duration>,
    is_access_only: bool,
) -> Result<user_access_key::Model, Box<dyn std::error::Error>> {
    let salt = SaltString::from_b64("bXlzYWx0bXlzYWx0").unwrap();
    let hashed_key = Argon2::default().hash_password(key, &salt)?;
    Ok(open_tab_entities::schema::user_access_key::Model {
        key_hash: hashed_key.to_string(),
        user_id,
        tournament_id,
        expiry_date: validity_duration.map(|d| (chrono::Utc::now() + d).naive_utc()),
        is_access_only
    })
}

pub async fn create_token_handler(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Json(request): Json<GetTokenRequest>,
) -> Result<Json<GetTokenResponse>, APIError> {
    if user.authorized_only_for_tournament.is_some() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Tournament specific tokens can't be used to create new keys",
        )
            .into());
    }
    if user.is_access_only {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Access only tokens can't create new tokens",
        )
            .into());
    }

    let key: [u8; 32] = thread_rng().gen::<[u8; 32]>();

    let duration = Duration::minutes(10);
    let expiration = if user.is_password_authorized { Some(duration) } else { None };
    let token = create_key(&key, user.uuid, request.tournament, expiration, !user.is_password_authorized).map_err(handle_error_dyn)?;
    token
        .into_active_model()
        .insert(&db)
        .await
        .map_err(handle_error)?;

    return Ok(GetTokenResponse {
        token: base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key),
        expires: expiration.map(|d| (chrono::Utc::now() + d).naive_utc().timestamp_millis()),
    }
    .into());
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterParticipantRequest {
    pub secret: String,
}

pub async fn register_user_handler(
    State(db): State<DatabaseConnection>,
    Json(RegisterParticipantRequest { secret }): Json<RegisterParticipantRequest>,
) -> Result<Json<RegisterUserResponse>, APIError> {
    let (participant_id, submitted_key) = Participant::decode_registration_key(secret)?;

    let participant = open_tab_entities::schema::participant::Entity::find_by_id(participant_id)
        .one(&db)
        .await
        .map_err(handle_error)?
        .ok_or(APIError::from((
            StatusCode::BAD_REQUEST,
            "Participant not found or key invalid",
        )))?;

    let db = db.begin().await.map_err(handle_error)?;

    match participant.registration_key {
        None => Err((StatusCode::BAD_REQUEST, "Participant can not be claimed").into()),
        Some(registration_key) => {
            if registration_key == submitted_key {
                let existing_user = open_tab_entities::schema::user_participant::Entity::find()
                    .filter(
                        open_tab_entities::schema::user_participant::Column::ParticipantId
                            .eq(participant_id),
                    )
                    .one(&db)
                    .await
                    .map_err(handle_error)?;

                if let Some(existing_user) = existing_user {
                    let key: [u8; 32] = thread_rng().gen::<[u8; 32]>();
                    let token =
                        create_key(&key, existing_user.user_id, None, None, false).map_err(handle_error_dyn)?;
                    token
                        .into_active_model()
                        .insert(&db)
                        .await
                        .map_err(handle_error)?;
                    db.commit().await.map_err(handle_error)?;
                    Ok(RegisterUserResponse {
                        user_id: existing_user.user_id,
                        participant_id: participant_id,
                        tournament_id: participant.tournament_id,
                        token: base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key),
                    }
                    .into())
                } else {
                    let new_user_id = Uuid::new_v4();
                    let new_user = open_tab_entities::schema::user::Model {
                        uuid: new_user_id,
                        password_hash: "".to_string(),
                        user_email: None,
                    };

                    new_user
                        .into_active_model()
                        .insert(&db)
                        .await
                        .map_err(handle_error)?;
                    let key: [u8; 32] = thread_rng().gen::<[u8; 32]>();
                    let user_key = create_key(&key, new_user_id, None, None, false).map_err(handle_error_dyn)?;
                    user_key
                        .into_active_model()
                        .insert(&db)
                        .await
                        .map_err(handle_error)?;

                    let user_participant = open_tab_entities::schema::user_participant::Model {
                        user_id: new_user_id,
                        participant_id,
                    };
                    user_participant
                        .into_active_model()
                        .insert(&db)
                        .await
                        .map_err(handle_error)?;

                    db.commit().await.map_err(handle_error)?;
                    Ok(RegisterUserResponse {
                        user_id: new_user_id,
                        participant_id,
                        tournament_id: participant.tournament_id,
                        token: base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key),
                    }
                    .into())
                }
            } else {
                db.rollback().await.map_err(handle_error)?;
                Err((StatusCode::BAD_REQUEST, "Incorrect key or participant id").into())
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RegistrationKeyInfo {
    participant_name: String,
    tournament_name: String,
}

pub async fn get_registration_info(
    State(db): State<DatabaseConnection>,
    Path(secret): Path<String>,
) -> Result<Json<RegistrationKeyInfo>, APIError> {
    let (participant_id, _) = Participant::decode_registration_key(secret)?;

    let (participant, tournament) = schema::participant::Entity::find_by_id(participant_id)
        .find_also_related(schema::tournament::Entity)
        .one(&db)
        .await
        .map_err(handle_error)?
        .ok_or(APIError::from((
            StatusCode::BAD_REQUEST,
            "Participant not found or key invalid",
        )))?;

    if tournament.is_none() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Participant is not related to tournament",
        )
            .into());
    }

    Ok(Json(RegistrationKeyInfo {
        participant_name: participant.name,
        tournament_name: tournament.unwrap().name,
    }))
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/users", post(create_user_handler))
        .route("/tokens", post(create_token_handler))
        .route("/register", post(register_user_handler))
        .route("/register/:secret", get(get_registration_info))
}
