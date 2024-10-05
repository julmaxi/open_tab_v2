

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, Router, routing::post, routing::patch, routing::get};

use base64::Engine;


use chrono::{NaiveDateTime, Utc};
use itertools::Itertools;
use open_tab_entities::domain::round::check_release_date;

use open_tab_entities::schema::{self, published_tournament, user_participant, user_tournament};
use open_tab_entities::{EntityGroup};
use rand::{thread_rng, Rng};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, IntoActiveModel, QueryOrder, QuerySelect};
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};
use tokio::join;

use crate::auth::{create_key, ExtractAuthenticatedUser, MaybeExtractAuthenticatedUser};
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
    // We need to create the tournament first, to set the first last_modified time
    let uuid: Uuid = request.uuid;
    let tournament = schema::tournament::ActiveModel {
        uuid: ActiveValue::Set(uuid),
        name: ActiveValue::Set(request.name.clone()),
        last_modified: ActiveValue::Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };
    tournament.insert(&db).await.map_err(handle_error)?;

    let mut changes = EntityGroup::new(uuid);
    let tournament = open_tab_entities::domain::tournament::Tournament {
        uuid,
        annoucements_password: Some("password".into()),
        name: request.name,
        ..Default::default()
    };
    changes.add(open_tab_entities::Entity::Tournament(tournament));

    // TODO: Prevent overriding tournament
    //changes.save_all_and_log_for_tournament(&db, uuid).await.map_err(handle_error_dyn)?;
    changes.save_all(&db).await?;
    let key = thread_rng().gen::<[u8; 32]>();
    let token = create_key(&key, user.uuid, Some(uuid), None, false).map_err(handle_error_dyn)?;
    token.into_active_model().insert(&db).await.map_err(handle_error)?;

    let user_tournament = open_tab_entities::schema::user_tournament::Model {
        user_id: user.uuid,
        tournament_id: uuid,
    };
    user_tournament.into_active_model().insert(&db).await.map_err(handle_error)?;

    return Ok(
        Json(
            CreateTournamentResponse {
                uuid,
                access_key: Some(base64::engine::general_purpose::STANDARD_NO_PAD.encode(&key)),
            }
        )
    )
}


/*
TODO: It would be nice to able to patch settings, instead of updating in bulk,
in particular for the image data.
*/
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TournamentPublicationSettings {
    pub public_name: String,
    pub image: Option<ImageInfo>,
    pub list_publicly: bool,
    pub show_participants: bool,
    pub show_motions: bool,
    pub show_draws: bool,
    pub show_tab: bool,
    pub start_date: Option<NaiveDateTime>,
    pub end_date: Option<NaiveDateTime>,
    pub location: Option<String>,
}

#[derive(Debug)]
pub struct ImageInfo {
    data: Vec<u8>,
    mime_type: String,
}

impl Serialize for ImageInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_str(&format!("data:{};base64,{}", self.mime_type, base64::engine::general_purpose::STANDARD.encode(&self.data)))
    }
}

impl<'de> Deserialize<'de> for ImageInfo {
    fn deserialize<D>(deserializer: D) -> Result<ImageInfo, D::Error> where D: serde::Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let parts = s.splitn(2, ',').collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom("Invalid image data"));
        }
        let mime_type = parts[0].splitn(2, ';').collect::<Vec<_>>()[0];
        let data = base64::engine::general_purpose::STANDARD.decode(parts[1]).map_err(serde::de::Error::custom)?;
        Ok(ImageInfo {
            data,
            mime_type: mime_type.to_string(),
        })
    }
}

impl From<open_tab_entities::schema::published_tournament::Model> for TournamentPublicationSettings {
    fn from(model: open_tab_entities::schema::published_tournament::Model) -> Self {
        let image = match (model.image_data, model.image_type) {
            (Some(data), Some(mime_type)) => {
                Some(
                    ImageInfo {
                        data,
                        mime_type,
                    }
                )
            }
            _ => None
        };
        Self {
            public_name: model.public_name,
            image,
            list_publicly: model.list_publicly,
            show_participants: model.show_participants,
            show_motions: model.show_motions,
            show_draws: model.show_draws,
            show_tab: model.show_tab,
            start_date: model.start_date,
            end_date: model.end_date,
            location: model.location,
        }
    }
}

pub async fn get_tournament_settings_handler(
    State(db) : State<DatabaseConnection>,
    ExtractAuthenticatedUser(user) : ExtractAuthenticatedUser,
    Path(tournament_id): Path<Uuid>,
) -> Result<Json<TournamentPublicationSettings>, APIError> {
    if !user.check_is_authorized_for_tournament_administration(&db, tournament_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized for this tournament"));
        return Err(err);
    }

    let published_tournament = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament_id))
        .one(&db)
        .await
        .map_err(handle_error)?;

    if let Some(published_tournament) = published_tournament {
        return Ok(Json(published_tournament.into()));
    }
    else {
        let mut default_settings = TournamentPublicationSettings::default();
        let tournament = open_tab_entities::schema::tournament::Entity::find()
            .filter(open_tab_entities::schema::tournament::Column::Uuid.eq(tournament_id))
            .one(&db)
            .await
            .map_err(handle_error)?;
        default_settings.public_name = tournament.map(|t| t.name).unwrap_or_default();

        return Ok(Json(default_settings));
    }
}

static UPLOAD_MB_LIMIT : usize = 2;

pub async fn update_tournament_settings_handler(
    State(db) : State<DatabaseConnection>,
    ExtractAuthenticatedUser(user) : ExtractAuthenticatedUser,
    Path(tournament_id): Path<Uuid>,
    Json(request): Json<TournamentPublicationSettings>,
) -> Result<(), APIError> {
    if !user.check_is_authorized_for_tournament_administration(&db, tournament_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized for this tournament"));
        return Err(err);
    }

    if let Some(image) = &request.image.as_ref() {
        if image.data.len() > 1024 * 1024 * UPLOAD_MB_LIMIT {
            let err = APIError::from((StatusCode::BAD_REQUEST, "Image data is too large"));
            return Err(err);
        }
    }

    let published_tournament = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament_id))
        .one(&db)
        .await
        .map_err(handle_error)?;

    let (mut published_tournament, should_insert) = match published_tournament {
        Some(published_tournament) => {
            (
                published_tournament.into_active_model(),
                false
            )
        }
        None => {
            (
                open_tab_entities::schema::published_tournament::ActiveModel {
                    uuid: sea_orm::ActiveValue::Set(Uuid::new_v4()),
                    tournament_id: sea_orm::ActiveValue::Set(Some(tournament_id)),
                    ..Default::default()
                },
                true
            )
        }
    };

    published_tournament.public_name = sea_orm::ActiveValue::Set(request.public_name);
    if let Some(image) = request.image {
        published_tournament.image_data = sea_orm::ActiveValue::Set(Some(image.data));
        published_tournament.image_type = sea_orm::ActiveValue::Set(Some(image.mime_type));
    }
    else {
        published_tournament.image_data = sea_orm::ActiveValue::Set(None);
        published_tournament.image_type = sea_orm::ActiveValue::Set(None);
    }
    published_tournament.list_publicly = sea_orm::ActiveValue::Set(request.list_publicly);
    published_tournament.show_participants = sea_orm::ActiveValue::Set(request.show_participants);
    published_tournament.show_motions = sea_orm::ActiveValue::Set(request.show_motions);
    published_tournament.show_draws = sea_orm::ActiveValue::Set(request.show_draws);
    published_tournament.show_tab = sea_orm::ActiveValue::Set(request.show_tab);
    published_tournament.start_date = sea_orm::ActiveValue::Set(request.start_date);
    published_tournament.end_date = sea_orm::ActiveValue::Set(request.end_date);
    published_tournament.location = sea_orm::ActiveValue::Set(request.location);

    if should_insert {
        published_tournament.insert(&db).await.map_err(handle_error)?;
    }
    else {
        published_tournament.update(&db).await.map_err(handle_error)?;
    }

    Ok(())
}

#[derive(Serialize)]
pub struct TournamentInfo {
    name: String,
    start_date: Option<NaiveDateTime>,
    end_date: Option<NaiveDateTime>,
    image: Option<ImageInfo>,
    show_tab: bool,
    show_participants: bool,
    user_is_participant: bool,
    
    tournament_uuid: Option<Uuid>,
}

impl From<open_tab_entities::schema::published_tournament::Model> for TournamentInfo {
    fn from(model: open_tab_entities::schema::published_tournament::Model) -> Self {
        let image = match (model.image_data, model.image_type) {
            (Some(data), Some(mime_type)) => {
                Some(
                    ImageInfo {
                        data,
                        mime_type,
                    }
                )
            }
            _ => None
        };

        let (show_tab, show_participants,) = if let Some(_) = model.tournament_id {
            (model.show_tab,model.show_participants)}
        else {
            (false,false)
        };

        Self {
            name: model.public_name,
            start_date: model.start_date,
            end_date: model.end_date,
            image,
            show_tab,
            show_participants,
            tournament_uuid: model.tournament_id,
            user_is_participant: false,
        }
    }
}

#[derive(Serialize)]
pub struct PublicTournamentsInfo {
    active_user: Vec<TournamentInfo>,
    active: Vec<TournamentInfo>,
    concluded: Vec<TournamentInfo>,
    upcoming: Vec<TournamentInfo>,
}

pub async fn get_active_tournaments_handler(
    State(db) : State<DatabaseConnection>,
    MaybeExtractAuthenticatedUser(user) : MaybeExtractAuthenticatedUser,
) -> Result<Json<PublicTournamentsInfo>, APIError> {
    let now = Utc::now().naive_utc();

    let active_tournaments = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(open_tab_entities::schema::published_tournament::Column::StartDate.lte(now).and(
            open_tab_entities::schema::published_tournament::Column::EndDate.gt(now).and(
                open_tab_entities::schema::published_tournament::Column::EndDate.is_null().not()
            )
        ))
        .order_by_desc(open_tab_entities::schema::published_tournament::Column::StartDate)
        .all(&db)
        .await
        .map_err(handle_error)?;

    let user_tournaments = if let Some(user) = user.as_ref() {
        open_tab_entities::schema::tournament::Entity::find()
            .join(sea_orm::JoinType::InnerJoin, user_participant::Relation::Participant.def().rev())
            .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::participant::Relation::Tournament.def().rev())
            .filter(open_tab_entities::schema::user_participant::Column::UserId.eq(user.uuid))
            .order_by_desc(open_tab_entities::schema::tournament::Column::LastModified)
            .limit(10)
            .all(&db).await.map_err(handle_error)?.into_iter().collect()
        }
    else {
        Vec::new()
    };

    let concluded_tournaments = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(
            open_tab_entities::schema::published_tournament::Column::EndDate.lt(now).and(
                open_tab_entities::schema::published_tournament::Column::EndDate.gt(now.checked_sub_signed(chrono::Duration::days(30)).unwrap()
            ))
        )    
        .order_by_desc(open_tab_entities::schema::published_tournament::Column::EndDate)
        .limit(10)
        .all(&db)
        .await
        .map_err(handle_error)?;

    let upcoming_tournaments = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(
            open_tab_entities::schema::published_tournament::Column::StartDate.gt(now)
        )
        .order_by_desc(open_tab_entities::schema::published_tournament::Column::StartDate)
        .limit(10)
        .all(&db)
        .await
        .map_err(handle_error)?;

    let concluded_ids = concluded_tournaments.iter().filter_map(|t| t.tournament_id).collect::<Vec<_>>();

    let active_tournaments_by_id = active_tournaments.iter().filter_map(|t| t.tournament_id.map(|uuid| (uuid, t))).collect::<HashMap<_, _>>();
    let user_tournaments = user_tournaments.into_iter().filter_map(|tournament| {
        if concluded_ids.contains(&tournament.uuid) {
            None
        }
        else {
            let published_tournament = active_tournaments_by_id.get(&tournament.uuid);

            if let Some(published_tournament) = published_tournament {
                let mut info = TournamentInfo::from((**published_tournament).clone());
                info.user_is_participant = true;
                Some(info)
            }
            else {
                let info = TournamentInfo {
                    name: tournament.name,
                    start_date: None,
                    end_date: None,
                    image: None,
                    show_tab: false,
                    show_participants: false,
                    user_is_participant: true,
                    tournament_uuid: Some(tournament.uuid),
                };

                Some(info)
            }
        }
    }).collect_vec();

    let user_tournament_ids = user_tournaments.iter().filter_map(|t| t.tournament_uuid).collect::<HashSet<_>>();

    Ok(Json(
        PublicTournamentsInfo {
            active_user: user_tournaments,
            active: active_tournaments.into_iter().filter(|p| p.tournament_id.map(|uuid| !user_tournament_ids.contains(&uuid)).unwrap_or(true)).map(|t| {
                t.into()
            }).collect(),
            concluded: concluded_tournaments.into_iter().map(|t| {
                t.into()
            }).collect(),
            upcoming: upcoming_tournaments.into_iter().map(|t| t.into()).collect(),
        }
    ))
}

#[derive(Serialize)]
pub struct TournamentPublicInfo {
    tournament_name: String,
    rounds: Vec<PublicRoundInfo>,
    show_motions: bool,
    show_tab: bool,
    show_draws: bool,
    show_participants: bool,
}

#[derive(Serialize)]
enum RoundState {
    InProgress,
    Concluded
}

#[derive(Serialize)]
pub struct PublicRoundInfo {
    uuid: Uuid,
    round_name: String,
    #[serde(flatten)]
    motion: Option<MotionInfo>,
    state: RoundState,
}

impl PublicRoundInfo {
    fn from_round(round: &open_tab_entities::schema::tournament_round::Model, show_motions: bool, now: NaiveDateTime) -> Self {
        Self {
            uuid: round.uuid,
            round_name: format!("Round {}", round.index + 1),
            motion: if show_motions && check_release_date(now, round.full_motion_release_time) { Some(MotionInfo::from_round(round)) } else { None },
            state: check_release_date(now, round.round_close_time).then(|| RoundState::Concluded).unwrap_or(RoundState::InProgress),
        }
    }
}

#[derive(Serialize)]
pub struct MotionInfo {
    motion: String,
    info_slide: Option<String>,
}

impl MotionInfo {
    fn from_round(round: &open_tab_entities::schema::tournament_round::Model) -> Self {
        Self {
            motion: round.motion.clone().unwrap_or("<Unknown Motion>".into()),
            info_slide: round.info_slide.clone(),
        }
    }
}

pub async fn get_public_tournament_info_handler(
    State(db) : State<DatabaseConnection>,
    Path(tournament_id): Path<Uuid>,
) -> Result<Json<TournamentPublicInfo>, APIError> {
    let now = Utc::now().naive_utc();

    let published_tournament = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament_id))
        .one(&db)
        .await
        .map_err(handle_error)?;

    if !published_tournament.is_some() {
        let err = APIError::from((StatusCode::NOT_FOUND, "Tournament not found"));
        return Err(err);
    }

    let published_tournament = published_tournament.unwrap();

    if !published_tournament.list_publicly {
        let err = APIError::from((StatusCode::FORBIDDEN, "This tournament is not public"));
        return Err(err);
    }

    // We will only ever show rounds with a released draw, either as active, or in the motion tab
    let all_rounds = open_tab_entities::schema::tournament_round::Entity::find()
        .filter(open_tab_entities::schema::tournament_round::Column::TournamentId.eq(tournament_id))
        .filter(open_tab_entities::schema::tournament_round::Column::DrawReleaseTime.lte(now))
        .order_by_asc(open_tab_entities::schema::tournament_round::Column::Index)
        .all(&db)
        .await
        .map_err(handle_error)?;

    let round_info = all_rounds.iter().map(|r| PublicRoundInfo::from_round(r, published_tournament.show_motions, now)).collect::<Vec<_>>();

    Ok(Json(
        TournamentPublicInfo {
            tournament_name: published_tournament.public_name,
            rounds: round_info,
            show_draws: published_tournament.show_draws,
            show_motions: published_tournament.show_motions,
            show_tab: published_tournament.show_tab,
            show_participants: published_tournament.show_participants,
        }
    ))
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/tournaments", post(create_tournament_handler))
        .route("/tournament/:tournament_id/settings", get(get_tournament_settings_handler))
        .route("/tournament/:tournament_id/settings", patch(update_tournament_settings_handler))
        .route("/public_tournaments", get(get_active_tournaments_handler))
        .route("/tournament/:tournament_id/public", get(get_public_tournament_info_handler))
}