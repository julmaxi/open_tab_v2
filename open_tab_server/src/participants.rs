use std::{collections::HashMap, error::Error, hash::Hash};

use axum::{extract::{Path, State}, Json, Router, routing::get};
use hyper::StatusCode;
use itertools::Itertools;
use open_tab_entities::domain::entity::LoadEntity;
use sea_orm::{DatabaseConnection, TransactionTrait, prelude::*, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::{response::{APIError, handle_error}, auth::{ExtractAuthenticatedUser, AuthenticatedUser, check_release_date}, state::AppState, tournament};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfoResponse {
    pub name: String,
    pub rounds: Vec<ParticipantRoundInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueInfo {
    uuid: Uuid,
    name: String
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantDebateInfo {
    uuid: Uuid,
    is_motion_released_to_non_aligned: bool,
    venue: Option<VenueInfo>
}

impl ParticipantDebateInfo {
    pub fn new_from(debate: open_tab_entities::schema::tournament_debate::Model, venue: Option<open_tab_entities::schema::tournament_venue::Model>) -> Self {
        Self {
            uuid: debate.uuid,
            is_motion_released_to_non_aligned: debate.is_motion_released_to_non_aligned,
            venue: venue.map(|v| VenueInfo{uuid: v.uuid, name: v.name})
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
pub enum RoundStatus {
    Planned,
    DrawReleased,
    InProgress,
    Completed
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantRoundInfo {
    pub uuid: Uuid,
    pub name: String,
    pub index: i32,
    pub participant_role: Option<ParticipantRoundRoleInfo>,
    pub motion: Motion,

    pub status: RoundStatus
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundTeamRole {
    Government,
    Opposition
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag="role")]
pub enum ParticipantRoundRoleInfo {
    NotDrawn,
    TeamSpeaker{
        debate: ParticipantDebateInfo,
        team_role: RoundTeamRole
    },
    NonAlignedSpeaker{debate: ParticipantDebateInfo, position: i32},
    Adjudicator{debate: ParticipantDebateInfo, position: i32},
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
        transaction.rollback().await.map_err(handle_error)?;
        return Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")));
    }

    let (participant, tournament) = participant_query_result.unwrap();
    let tournament = tournament.unwrap(); // Guaranteed by consistency constraints

    let has_access = user.check_is_authorized_for_tournament_administration(&transaction, tournament.uuid).await?;

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
        transaction.rollback().await.map_err(handle_error)?;
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
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::ballot_adjudicator::Column::AdjudicatorId.eq(participant_id)
    ).all(&transaction).await.map_err(handle_error)?;

    let participant_non_aligned_speaker_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotSpeech.def())
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::ballot_speech::Column::SpeakerId.eq(participant_id).and(
            open_tab_entities::schema::ballot_speech::Column::Role.eq(
                open_tab_entities::domain::ballot::SpeechRole::NonAligned.to_str()
            )
        )
    ).all(&transaction).await.map_err(handle_error)?;

    //FIXME: Unelegant
    let participant_gov_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotTeam.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot_team::Relation::Team.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::team::Relation::Speaker.def())
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::speaker::Column::Uuid.eq(participant_id).and(
            open_tab_entities::schema::ballot_team::Column::Role.eq(
                open_tab_entities::domain::ballot::SpeechRole::Government.to_str()
            )
        )
    ).all(&transaction).await.map_err(handle_error)?;

    let participant_opp_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotTeam.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot_team::Relation::Team.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::team::Relation::Speaker.def())
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::speaker::Column::Uuid.eq(participant_id).and(
            open_tab_entities::schema::ballot_team::Column::Role.eq(
                open_tab_entities::domain::ballot::SpeechRole::Opposition.to_str()
            )
        )
    ).all(&transaction).await.map_err(handle_error)?;

    let all_ballot_ids = participant_adjudicator_debates.iter().map(|d| d.0.ballot_id)
    .chain(participant_non_aligned_speaker_debates.iter().map(|d| d.0.ballot_id))
    .chain(participant_gov_debates.iter().map(|d| d.0.ballot_id))
    .chain(participant_opp_debates.iter().map(|d| d.0.ballot_id))
    .collect::<Vec<_>>();
    let ballot_map = open_tab_entities::domain::ballot::Ballot::get_many(&transaction, all_ballot_ids).await?.into_iter().map(
        |b| (b.uuid, b)
    ).collect::<HashMap<_, _>>();

    let participant_adjudicator_debates = participant_adjudicator_debates.into_iter().map(
        |(d, v)| {
            let ballot = ballot_map.get(&d.ballot_id).unwrap();
            (
                d.round_id,
                ParticipantRoundRoleInfo::Adjudicator{
                    debate: ParticipantDebateInfo::new_from(d, v),
                    position: ballot.adjudicators.iter().position(|a| *a == participant_id).unwrap() as i32
                }
            )
        }
    );
    let participant_non_aligned_speaker_debates = participant_non_aligned_speaker_debates.into_iter().map(
        |(d, v)| {
            let ballot = ballot_map.get(&d.ballot_id).unwrap();
            let position = ballot.speeches.iter().position(|s| s.speaker == Some(participant_id)).unwrap() as i32;
            (d.round_id, ParticipantRoundRoleInfo::NonAlignedSpeaker {
                debate: ParticipantDebateInfo::new_from(d, v),
                position
            })
        }
    );
    let participant_gov_debates = participant_gov_debates.into_iter().map(
        |(d, v)| {
            (d.round_id, ParticipantRoundRoleInfo::TeamSpeaker{debate: ParticipantDebateInfo::new_from(d, v), team_role: RoundTeamRole::Government })
        }
    );
    let participant_opp_debates = participant_opp_debates.into_iter().map(
        |(d, v)| (d.round_id, ParticipantRoundRoleInfo::TeamSpeaker{debate: ParticipantDebateInfo::new_from(d, v), team_role: RoundTeamRole::Opposition })
    );

    let round_roles : HashMap<Uuid, Vec<ParticipantRoundRoleInfo>> = participant_adjudicator_debates.chain(participant_non_aligned_speaker_debates).chain(participant_gov_debates).chain(participant_opp_debates).into_grouping_map().collect();

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
                ParticipantRoundRoleInfo::NonAlignedSpeaker{debate, ..} => check_release_date(current_time, round.team_motion_release_time) && debate.is_motion_released_to_non_aligned,
                _ => false
            };

            let status = if check_release_date(current_time, round.round_close_time) {
                RoundStatus::Completed
            } else if check_release_date(current_time, round.team_motion_release_time) {
                RoundStatus::DrawReleased
            } else if check_release_date(current_time, round.draw_release_time) {
                RoundStatus::InProgress
            } else {
                RoundStatus::Planned
            };

            ParticipantRoundInfo {
                uuid: round.uuid,
                index: round.index,
                name: format!("Round {}", round.index + 1),
                participant_role: if check_release_date(current_time, round.draw_release_time) { Some(role) } else { None },
                motion: if show_motion {
                    Motion::Shown{motion: round.motion.unwrap_or("<Missing Motion>".into()), info_slide: round.info_slide}
                } else {
                    Motion::Hidden
                },
                status
            }
        }
    ).sorted_by_key(|info| info.index).collect_vec();

    transaction.rollback().await.map_err(handle_error)?;
    Ok(Json(ParticipantInfoResponse {
        name: participant.name,
        rounds
    }))
}


pub fn router() -> Router<AppState> {
    Router::new()
    .route("/participant/:participant_id", get(get_participant_info))
}