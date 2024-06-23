
use std::collections::HashMap;
use std::str::FromStr;

use axum::{Router, Json};
use axum::extract::{Path, State};
use axum::routing::{post, get, patch};
use chrono::Utc;
use open_tab_entities::domain::ballot_speech_timing::BallotSpeechTiming;
use std::sync::{Arc};


use open_tab_entities::domain::entity::LoadEntity;

use open_tab_entities::{prelude::*, domain};
use open_tab_entities::domain::ballot::{BallotParseError};
use open_tab_entities::schema::{self};
use sea_orm::{prelude::*};
use serde::{Serialize, Deserialize};

use itertools::Itertools;
use tokio::sync::{RwLock};

use crate::auth::{AuthenticatedUser, ExtractAuthenticatedUser};
use crate::ballot::check_is_authorized_for_debate_result_submission;
use crate::notify::{ParticipantNotificationManager, ParticipantEvent, ParticipantEventType, DebateCurrentSpeech};
use crate::patch::PatchValue;


use open_tab_entities::domain::round::check_release_date;

use crate::response::{APIError, handle_error};
use crate::state::AppState;


#[derive(Debug, Serialize, Deserialize)]
#[serde(tag="state")]
enum UpdateDebateStateRequest {
    NonAlignedMotionRelease{release: bool}
}


async fn update_debate_state(
    State(db): State<DatabaseConnection>,
    State(notifications): State<Arc<RwLock<ParticipantNotificationManager>>>,
    Path(debate_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Json(request): Json<UpdateDebateStateRequest>,
) -> Result<(), APIError> {
    if !check_is_authorized_for_debate_result_submission(&db, &user, debate_id).await? {
        return  Err((axum::http::StatusCode::FORBIDDEN, "Not authorized for debate"))?;
    }

    let mut query_results = schema::tournament_debate::Entity::find_by_id(debate_id).find_with_related(schema::tournament_round::Entity).all(&db).await.map_err(
        |_| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Error getting debate")
        }
    )?;
    if query_results.len() != 1 {
        return  Err((axum::http::StatusCode::NOT_FOUND, "Debate not found"))?;
    }

    let (debate, mut rounds) = query_results.pop().unwrap();
    if rounds.len() != 1 {
        return Err((axum::http::StatusCode::NOT_FOUND, "Round not found"))?;
    }
    let round = rounds.pop().unwrap();
    let debate_has_started = round.debate_start_time.map_or(false, |t| t <= Utc::now().naive_utc());

    if !debate_has_started {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Debate has not started"))?;
    }

    let debate = domain::debate::TournamentDebate::from_model(debate);

    let mut entities = EntityGroup::new();

    match request {
        UpdateDebateStateRequest::NonAlignedMotionRelease{release} => {
            entities.add(Entity::TournamentDebate(domain::debate::TournamentDebate {
                is_motion_released_to_non_aligned: release,
                ..debate
            }));
        }
    }

    entities.save_all_and_log_for_tournament(&db, round.tournament_id).await?;

    notifications.as_ref().read().await.notify_debate_non_aligned_motion_release_state(&db, debate_id).await?;

    Ok(())
}


async fn get_debate_timing_info(
    State(db): State<DatabaseConnection>,
    Path(debate_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
) -> Result<Json<DebateTimingStateResponse>, APIError> {
    let mut debate = schema::tournament_debate::Entity::find_by_id(debate_id).find_with_related(schema::tournament_round::Entity).all(&db).await.map_err(
        handle_error
    )?;
    if debate.len() != 1{
        return  Err((axum::http::StatusCode::NOT_FOUND, "Debate not found"))?;
    }
    let (debate, mut rounds) = debate.pop().unwrap();
    let round = rounds.pop().expect("Guaranteed by db constraints");

    if !user.check_is_authorized_in_tournament(&db, round.tournament_id).await? {
        return Err((axum::http::StatusCode::FORBIDDEN, "Not authorized for tournament"))?;
    }

    let round = domain::round::TournamentRound::from_model(round);
    let now = Utc::now().naive_utc();
    if !check_release_date(now, round.draw_release_time) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Draw not released"))?;
    }

    let ballot = domain::ballot::Ballot::get(&db, debate.ballot_id).await?;


    let timing_info_by_speech_ids = BallotSpeechTiming::get_all_in_debate(&db, debate_id).await?.into_iter().map(|timing| {
        Ok(((SpeechRole::from_str(&timing.speech_role)?, timing.speech_position as u8), timing))
    }).collect::<Result<HashMap<_, _>, BallotParseError>>().map_err(handle_error)?;

    let speeches = ballot.speeches.into_iter().flat_map(|speech| {
        let mut out = vec![];
        let timing = timing_info_by_speech_ids.get(&(speech.role.clone(), speech.position)).cloned();

        let (target_length, response_target_length, segments) = match &speech.role {
            //Ring times are cummulative
            SpeechRole::Government | SpeechRole::Opposition => (
                7 * 60,
                None,
                vec![
                    SegmentInfo {duration: 60, end_ring: RingType::Single, segment_type: SegmentType::Protected},
                    SegmentInfo {duration: 5*60, end_ring: RingType::Single, segment_type: SegmentType::Normal},
                    SegmentInfo {duration: 60, end_ring: RingType::Double, segment_type: SegmentType::Protected},
                    SegmentInfo {duration: 15, end_ring: RingType::Permanent, segment_type: SegmentType::Grace},
                ]
            ),
            SpeechRole::NonAligned => (
                3 * 60 + 30,
                Some(60),
                vec![
                    SegmentInfo {duration: 60, end_ring: RingType::Single, segment_type: SegmentType::Protected},
                    SegmentInfo {duration: 2 * 60, end_ring: RingType::Single, segment_type: SegmentType::Normal},
                    SegmentInfo {duration: 30, end_ring: RingType::Double, segment_type: SegmentType::Protected},
                    SegmentInfo {duration: 15, end_ring: RingType::Permanent, segment_type: SegmentType::Grace},
                ]
        ),
        };

        out.push(
            DebateSpeechTiming {
                start: timing.as_ref().map(|timing| timing.start_time).flatten(),
                end: timing.as_ref().map(|timing| timing.end_time).flatten(),
                target_length,
                role: speech.role,
                position: speech.position,
                segments,
                is_response: false,
                pause_milliseconds: timing.as_ref().map(|timing| timing.pause_milliseconds).unwrap_or(0)
            }
        );

        if let Some(response_target_length) = response_target_length {
            out.push(
                DebateSpeechTiming {
                    start: timing.as_ref().map(|timing| timing.response_start_time).flatten(),
                    end: timing.as_ref().map(|timing| timing.response_end_time).flatten(),
                    target_length: response_target_length,
                    role: speech.role,
                    position: speech.position,
                    segments: vec![
                        SegmentInfo {
                            duration: response_target_length,
                            end_ring: RingType::Double,
                            segment_type: SegmentType::Protected
                        }
                    ],
                    is_response: true,
                    pause_milliseconds: timing.as_ref().map(|timing| timing.response_pause_milliseconds).unwrap_or(0)
                }
            );
        }

        out
    }).collect_vec();
    Ok(
        Json(DebateTimingStateResponse {
            speeches,
            participant_may_control: check_is_authorized_for_debate_result_submission(&db, &user, debate_id).await?
        })
    )
}


#[derive(Deserialize, Debug)]
struct DebateTimingUpdateRequest {
    speech_role: SpeechRole,
    speech_position: u8,
    #[serde(default)]
    start: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    end: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    response_start: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    response_end: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    pause_milliseconds: PatchValue<i32>,
    #[serde(default)]
    response_pause_milliseconds: PatchValue<i32>,
}

async fn check_has_permission_for_timer<C>(
    db: &C,
    debate_id: Uuid,
    user: &AuthenticatedUser
) -> Result<(Uuid, Uuid), APIError> where C: ConnectionTrait {
    //Returns ballot and tournament id if successful for efficiency
    let mut debate = schema::tournament_debate::Entity::find_by_id(debate_id).find_with_related(schema::tournament_round::Entity).all(db).await.map_err(
        handle_error
    )?;
    if debate.len() != 1{
        return  Err((axum::http::StatusCode::NOT_FOUND, "Debate not found"))?;
    }
    let (debate, mut rounds) = debate.pop().unwrap();
    let round = rounds.pop().expect("Guaranteed by db constraints");

    if !check_is_authorized_for_debate_result_submission(db, user, debate_id).await? {
        return Err((axum::http::StatusCode::FORBIDDEN, "Not authorized for tournament"))?;
    }

    let round = domain::round::TournamentRound::from_model(round);
    let now = Utc::now().naive_utc();
    if !check_release_date(now, round.draw_release_time) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Draw not released"))?;
    }

    Ok((debate.ballot_id, round.tournament_id))
}

async fn set_debate_timing(
    State(db): State<DatabaseConnection>,
    Path(debate_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    State(notifications): State<Arc<RwLock<ParticipantNotificationManager>>>,
    Json(request): Json<DebateTimingUpdateRequest>,
) -> Result<(), APIError> {

    let (ballot_id, tournament_id) = check_has_permission_for_timer(&db, debate_id, &user).await?;

    let timing = BallotSpeechTiming::get_from_speech(&db, ballot_id, request.speech_role, request.speech_position as i32).await?;
    let mut timing = timing.unwrap_or(BallotSpeechTiming {
        uuid: Uuid::new_v4(),
        speech_ballot_id: ballot_id,
        speech_role: request.speech_role.to_str().to_string(),
        speech_position: request.speech_position as i32,
        start_time: None,
        end_time: None,
        response_start_time: None,
        response_end_time: None,
        pause_milliseconds: 0,
        response_pause_milliseconds: 0
    });

    let mut did_update_main = false;
    if let PatchValue::Set(start) = request.start {
        timing.start_time = start;
        did_update_main = true;
    }
    if let PatchValue::Set(end) = request.end {
        timing.end_time = end;
        did_update_main = true;
    }
    if let PatchValue::Set(pause) = request.pause_milliseconds {
        timing.pause_milliseconds = pause;
        did_update_main = true;
    }

    let mut did_update_response = false;
    if let PatchValue::Set(response_start) = request.response_start {
        timing.response_start_time = response_start;
        did_update_response = true;
    }
    if let PatchValue::Set(response_end) = request.response_end {
        timing.response_end_time = response_end;
        did_update_response = true;
    }
    if let PatchValue::Set(response_pause) = request.response_pause_milliseconds {
        timing.response_pause_milliseconds = response_pause;
        did_update_response = true;
    }

    let mut events = vec![];

    if did_update_main {
        events.push(
            ParticipantEvent {
                event: ParticipantEventType::SpeechTimeUpdate { 
                    speech_role: request.speech_role,
                    speech_position: timing.speech_position,
                    start: timing.start_time,
                    end: timing.end_time,
                    is_response: false,
                    pause_milliseconds: timing.pause_milliseconds
                }
            }
        );
    }

    if did_update_response {
        events.push(
            ParticipantEvent {
                event: ParticipantEventType::SpeechTimeUpdate { 
                    speech_role: request.speech_role,
                    speech_position: timing.speech_position,
                    start: timing.response_start_time,
                    end: timing.response_end_time,
                    is_response: true,
                    pause_milliseconds: timing.response_pause_milliseconds
                }
            }
        );
    }


    let mut group = EntityGroup::new();
    group.add(Entity::BallotSpeechTiming(timing));
    group.save_all_and_log_for_tournament(&db, tournament_id).await?;


    for event in events {
        notifications.read().await.notify_debate(&db, debate_id, event).await?;
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
struct DebateCurrentSpeechNotificationRequest {
    speech: Option<DebateCurrentSpeech>,
}

async fn send_current_speech_notification(
    State(db): State<DatabaseConnection>,
    Path(debate_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    State(notifications): State<Arc<RwLock<ParticipantNotificationManager>>>,
    Json(request): Json<DebateCurrentSpeechNotificationRequest>,
) -> Result<(), APIError> {
    let _ = check_has_permission_for_timer(&db, debate_id, &user).await?;

    notifications.read().await.notify_debate(
        &db,
        debate_id,
        ParticipantEvent {
            event: ParticipantEventType::ActiveSpeechUpdate {
                speech: request.speech
            }
        }
    ).await?;
    

    Ok(())
}

#[derive(Serialize)]
struct DebateTimingStateResponse {
    speeches: Vec<DebateSpeechTiming>,
    participant_may_control: bool
}

#[derive(Serialize)]
struct DebateSpeechTiming {
    role: SpeechRole,
    position: u8,
    is_response: bool,

    start: Option<chrono::NaiveDateTime>,
    end: Option<chrono::NaiveDateTime>,
    target_length: u64,
    segments: Vec<SegmentInfo>,

    pause_milliseconds: i32,
}

#[derive(Serialize)]
struct SegmentInfo {
    duration: u64,
    end_ring: RingType,
    segment_type: SegmentType
}

#[derive(Serialize)]
enum SegmentType {
    Protected,
    Normal,
    Grace
}

#[derive(Serialize)]
enum RingType {
    Single,
    Double,
    Permanent
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/debate/:debate_id/state", post(update_debate_state)
        ).route(
            "/debate/:debate_id/timing", get(get_debate_timing_info)
        ).route(
            "/debate/:debate_id/timing", patch(set_debate_timing)
        ).route(
            "/debate/:debate_id/timing/notify", post(send_current_speech_notification)
        )
}