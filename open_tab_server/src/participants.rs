use std::{collections::HashMap, error::Error};

use axum::{extract::{Path, State}, Json, Router, routing::get};
use hyper::StatusCode;
use itertools::Itertools;
use sea_orm::{DatabaseConnection, TransactionTrait, prelude::*, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::{response::{APIError, handle_error}, auth::{ExtractAuthenticatedUser, AuthenticatedUser, check_release_date}, state::AppState, tournament};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfoResponse {
    pub rounds: Vec<ParticipantRoundInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantDebateInfo {
    is_motion_released_to_non_aligned: bool
}

impl From<open_tab_entities::schema::tournament_debate::Model> for ParticipantDebateInfo {
    fn from(model: open_tab_entities::schema::tournament_debate::Model) -> Self {
        Self {
            is_motion_released_to_non_aligned: model.is_motion_released_to_non_aligned
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum Motion {
    Hidden,
    Shown{motion: String, info_slide: Option<String>}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantRoundInfo {
    pub uuid: Uuid,
    pub index: i32,
    pub participant_role: Option<ParticipantRoundRoleInfo>,
    pub motion: Motion
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRoundRoleInfo {
    NotDrawn,
    TeamSpeaker{debate: ParticipantDebateInfo},
    NonAlignedSpeaker{debate: ParticipantDebateInfo},
    Adjudicator{debate: ParticipantDebateInfo},
    Multiple
}

async fn get_participant_info(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<ParticipantInfoResponse>, APIError> {
    let transaction = db.begin().await.map_err(handle_error)?;

    let participant_query_result = open_tab_entities::schema::participant::Entity::find_by_id(participant_id)
    .find_also_related(open_tab_entities::schema::tournament::Entity)
        .one(&transaction).await.map_err(handle_error)?;

    if participant_query_result.is_none() {
        return Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")));
    }

    let (participant, tournament) = participant_query_result.unwrap();
    let tournament = tournament.unwrap(); // Guaranteed by consistency constraints

    let has_access = user.check_is_authorized_for_tournament_administration(&db, tournament.uuid).await?;

    let has_access = match has_access {
        true => true,
        false => {
            open_tab_entities::schema::user_participant::Entity::find()
            .filter(
                open_tab_entities::schema::user_participant::Column::UserId.eq(user.uuid)
            ).one(&transaction).await.map_err(handle_error)?.is_some()
        }
    };

    if !has_access {
        return Err(APIError::from((StatusCode::FORBIDDEN, "You do not have access to this participant")));
    }

    let all_rounds = open_tab_entities::schema::tournament_round::Entity::find()
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::tournament_round::Relation::Tournament.def())
    .join(sea_orm::JoinType::LeftJoin, open_tab_entities::schema::participant::Relation::Tournament.def().rev())
    .filter(
        open_tab_entities::schema::participant::Column::Uuid.eq(participant_id)
    ).all(&transaction).await.map_err(handle_error)?;


    let participant_adjudicator_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotAdjudicator.def())
    .filter(
        open_tab_entities::schema::ballot_adjudicator::Column::AdjudicatorId.eq(participant_id)
    ).all(&transaction).await.map_err(handle_error)?;

    let participant_non_aligned_speaker_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotSpeech.def())
    .filter(
        open_tab_entities::schema::ballot_speech::Column::SpeakerId.eq(participant_id).and(
            open_tab_entities::schema::ballot_speech::Column::Role.eq(
                open_tab_entities::domain::ballot::SpeechRole::NonAligned.to_str()
            )
        )
    ).all(&transaction).await.map_err(handle_error)?;
    
    let participant_team_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotTeam.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot_team::Relation::Team.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::team::Relation::Speaker.def())
    .filter(
        open_tab_entities::schema::speaker::Column::Uuid.eq(participant_id)
    ).all(&transaction).await.map_err(handle_error)?;

    let participant_adjudicator_debates = participant_adjudicator_debates.into_iter().map(
        |d| (d.round_id, ParticipantRoundRoleInfo::Adjudicator{debate: d.into()})
    );
    let participant_non_aligned_speaker_debates = participant_non_aligned_speaker_debates.into_iter().map(
        |d| (d.round_id, ParticipantRoundRoleInfo::NonAlignedSpeaker {debate: d.into()})
    );
    let participant_team_debates = participant_team_debates.into_iter().map(
        |d| (d.round_id, ParticipantRoundRoleInfo::TeamSpeaker{debate: d.into()})
    );

    let round_roles : HashMap<Uuid, Vec<ParticipantRoundRoleInfo>> = participant_adjudicator_debates.chain(participant_non_aligned_speaker_debates).chain(participant_team_debates).into_grouping_map().collect();

    let current_time = chrono::Utc::now().naive_utc();

    let rounds = all_rounds.into_iter().map(
        |round| {
            let role = match round_roles.get(&round.uuid) {
                Some(roles) => {
                    if roles.len() == 1 {
                        roles[0].clone()
                    } else {
                        ParticipantRoundRoleInfo::Multiple
                    }
                },
                None => ParticipantRoundRoleInfo::NotDrawn
            };

            let show_motion = check_release_date(current_time, round.full_motion_release_time) || match &role {
                ParticipantRoundRoleInfo::Adjudicator{..} | ParticipantRoundRoleInfo::TeamSpeaker{..} => 
                check_release_date(current_time, round.team_motion_release_time),
                ParticipantRoundRoleInfo::NonAlignedSpeaker{debate} => check_release_date(current_time, round.team_motion_release_time) && debate.is_motion_released_to_non_aligned,
                _ => false
            };

            ParticipantRoundInfo {
                uuid: round.uuid,
                index: round.index,
                participant_role: if check_release_date(current_time, round.draw_release_time) { Some(role) } else { None },
                motion: if show_motion {
                    Motion::Shown{motion: round.motion.unwrap_or("<Missing Motion>".into()), info_slide: round.info_slide}
                } else {
                    Motion::Hidden
                }
            }
        }
    ).collect_vec();

    transaction.rollback().await.map_err(handle_error)?;
    Ok(Json(ParticipantInfoResponse {
        rounds
    }))
}


pub fn router() -> Router<AppState> {
    Router::new()
    .route("/participant/:participant_id", get(get_participant_info))
}