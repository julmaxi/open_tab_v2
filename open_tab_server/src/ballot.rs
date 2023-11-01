
use std::collections::hash_map::RandomState;
use std::{collections::HashMap, error::Error};

use axum::{Router, Json};
use axum::extract::{Path, State};
use axum::routing::{post, get};
use chrono::Utc;
use hyper::StatusCode;
use open_tab_entities::domain::debate_backup_ballot::DebateBackupBallot;
use open_tab_entities::domain::entity::LoadEntity;
use open_tab_entities::domain::round::RoundState;
use open_tab_entities::prelude::*;
use open_tab_entities::domain::{ballot::Ballot};
use open_tab_entities::schema::{self};
use sea_orm::{prelude::*, JoinType, QuerySelect, TransactionTrait};
use serde::{Serialize, Deserialize};

use itertools::Itertools;

use crate::auth::{AuthenticatedUser, ExtractAuthenticatedUser, check_release_date};
use crate::response::APIError;
use crate::state::AppState;
use crate::tournament;

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayBallot {
    pub uuid: Uuid,

    pub adjudicators: Vec<DisplayAdjudicator>,
    pub president: Option<DisplayAdjudicator>,
    pub government: DisplayBallotTeam,
    pub opposition: DisplayBallotTeam,

    pub speeches: Vec<DisplayBallotSpeech>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayAdjudicator {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DisplayBallotTeam {
    pub uuid: Uuid,
    pub name: String,
    pub members: Vec<DisplaySpeaker>,
    pub scores: HashMap<Uuid, i16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplaySpeaker {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayBallotSpeech {
    pub scores: HashMap<Uuid, i16>,
    pub speaker: Option<DisplaySpeaker>,
    pub position: u8,
    pub role: SpeechRole
}

impl DisplayBallot {
    async fn from_id<C>(ballot_id: Uuid, db: &C) -> Result<Self, anyhow::Error> where C: ConnectionTrait {
        let ballot = Ballot::get(db, ballot_id).await?;

        let teams = ballot.government.team.iter().chain(ballot.opposition.team.iter()).map(|u| *u).collect_vec();
    
        let team_name_map = if teams.len() > 0 {
            let teams = schema::team::Entity::find()
            .filter(schema::team::Column::Uuid.is_in(teams.clone()))
            .all(db)
            .await?;
            teams.into_iter().map(|t| (t.uuid, t.name)).collect()
        }
        else {
            HashMap::new()
        };
    
        let team_members = schema::participant::Entity::find()
            .find_also_related(schema::speaker::Entity)
            .filter(schema::speaker::Column::TeamId.is_in(teams.clone()))
            .all(db)
            .await?;
    
        let adjudicators = schema::participant::Entity::find()
            .filter(schema::participant::Column::Uuid.is_in(ballot.adjudicators.clone()))
            .all(db)
            .await?;
    
        let speech_speakers = schema::participant::Entity::find()
            .join_rev(
                JoinType::InnerJoin,
                schema::ballot_speech::Entity::belongs_to(schema::participant::Entity).from(schema::ballot_speech::Column::SpeakerId).to(schema::participant::Column::Uuid)
                .into(),
            )
            .filter(schema::ballot_speech::Column::BallotId.eq(ballot_id).and(
                schema::ballot_speech::Column::SpeakerId.is_in(ballot.speeches.iter().filter_map(|s| s.speaker))
            ))
            .all(db)
            .await?;
    
        let participant_name_map : HashMap<Uuid, String, RandomState> = HashMap::from_iter(
            team_members.iter().map(|p| (p.0.uuid, p.0.name.clone())).chain(
                speech_speakers.iter().map(|p| (p.uuid, p.name.clone()))
            ).chain(
                adjudicators.iter().map(|a| (a.uuid, a.name.clone()))
            )
        );
    
        let gov_members = team_members.iter().filter(
            |p| {
                if let Some(gov_id) = ballot.government.team {
                    p.1.as_ref().map(|p| p.team_id).flatten().unwrap_or(Uuid::nil()) == gov_id
                }
                else {
                    false
                }
            }
        ).map(|p| DisplaySpeaker{uuid: p.0.uuid, name: p.0.name.clone()}).collect_vec();
        let opp_members = team_members.iter().filter(
            |p| {
                if let Some(opp_id) = ballot.opposition.team {
                    p.1.as_ref().map(|p| p.team_id).flatten().unwrap_or(Uuid::nil()) == opp_id
                }
                else {
                    false
                }
            }
        ).map(|p| DisplaySpeaker{uuid: p.0.uuid, name: p.0.name.clone()}).collect_vec();
    
        Ok(DisplayBallot {
            uuid: ballot.uuid,
            adjudicators: ballot.adjudicators.iter().map(|a| DisplayAdjudicator{uuid: *a, name: participant_name_map.get(&a).unwrap_or(&"Unknown".to_string()).clone()}).collect_vec(),
            government: ballot.government.team.as_ref().map(
                |team_id| DisplayBallotTeam {
                    uuid: *team_id,
                    name: team_name_map.get(team_id).unwrap_or(&"Unknown".to_string()).clone(),
                    members: gov_members,
                    scores: ballot.government.scores.iter().map(|(adj, score)| {
                        (*adj, score.total())
                    }).collect(),
                }
            ).unwrap_or(Default::default()),
            opposition: ballot.opposition.team.as_ref().map(
                |team_id| DisplayBallotTeam {
                    uuid: *team_id,
                    name: team_name_map.get(team_id).unwrap_or(&"Unknown".to_string()).clone(),
                    members: opp_members,
                    scores: ballot.opposition.scores.iter().map(|(adj, score)| {
                        (*adj, score.total())
                    }).collect(),
                }
            ).unwrap_or(Default::default()),
            speeches: ballot.speeches.iter().map(|s| {
                DisplayBallotSpeech {
                    scores: s.scores.iter().map(|(adj, score)| {
                        (*adj, score.total())
                    }).collect(),
                    speaker: s.speaker.as_ref().map(|speaker_id| DisplaySpeaker {
                        uuid: *speaker_id,
                        name: participant_name_map.get(speaker_id).unwrap_or(&"Unknown".to_string()).clone(),
                    }),
                    role: s.role.clone(),
                    position: s.position,
                }
            }).collect_vec(),
            president: ballot.president.as_ref().map(|president_id| DisplayAdjudicator {
                uuid: *president_id,
                name: participant_name_map.get(president_id).unwrap_or(&"Unknown".to_string()).clone(),
            }),
        })
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct GetDebateResponse {
    pub ballot: DisplayBallot,    
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetBallotSubmissionResponse {
    pub debate_id: Uuid,
    pub ballot: DisplayBallot
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitBallotRequest {
    pub ballot: Ballot
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitBallotResponse {
    pub submission_id: Uuid,
    pub ballot_id: Uuid
}

async fn check_is_authorized_for_debate_result_submission<C>(
    db: &C,
    user: &AuthenticatedUser,
    debate_id: Uuid,
) -> Result<bool, anyhow::Error> where C: ConnectionTrait {
    let r = schema::tournament_debate::Entity::find_by_id(debate_id)
    .find_with_related(schema::tournament_round::Entity).all(db).await?;

    if r.len() != 1 {
        return Ok(false);
    }
    let r = r.into_iter().next().unwrap();
    //let debate = r.0;
    let round = r.1.into_iter().next().unwrap();

    if user.check_is_authorized_for_tournament_administration(db, round.tournament_id).await? {
        return Ok(true);
    }

    if !check_release_date(chrono::Utc::now().naive_utc(), round.draw_release_time) {
        return Ok(false);
    }

    let debate_adjudicator = schema::ballot_adjudicator::Entity::find()
    .inner_join(schema::ballot::Entity)
    .inner_join(schema::adjudicator::Entity)
    .join(
        JoinType::InnerJoin,
        schema::adjudicator::Relation::Participant.def()
    )
    .join(
        JoinType::InnerJoin,
        schema::user_participant::Relation::Participant.def().rev()
    ).join(
        JoinType::InnerJoin,
        schema::user_participant::Relation::User.def()
    ).join(
        JoinType::InnerJoin,
        schema::tournament_debate::Relation::Ballot.def().rev()
    ).filter(
        schema::user::Column::Uuid.eq(user.uuid).and(
            schema::tournament_debate::Column::Uuid.eq(debate_id)
        )
    ).one(db).await?;

    return Ok(debate_adjudicator.is_some());
}

async fn get_ballot_submission(
    State(db): State<DatabaseConnection>,
    Path(submission_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser
) -> Result<Json<GetBallotSubmissionResponse>, APIError> {
    let submission = schema::debate_backup_ballot::Entity::find_by_id(submission_id).one(&db).await.map_err(
        |_| {
            (hyper::StatusCode::INTERNAL_SERVER_ERROR, "Error getting ballot submission")
        }
    )?;
    let submission = submission.ok_or((hyper::StatusCode::NOT_FOUND, "Submission not found"))?;


    if !check_is_authorized_for_debate_result_submission(&db, &user, submission.debate_id).await? {
        return  Err((hyper::StatusCode::FORBIDDEN, "Not authorized for debate"))?;
    }

    let ballot = DisplayBallot::from_id(submission.ballot_id, &db).await?;

    Ok(Json(GetBallotSubmissionResponse {
        debate_id: submission.debate_id,
        ballot
    }))
}

async fn get_debate(
    State(db): State<DatabaseConnection>,
    Path(debate_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser
) -> Result<Json<GetDebateResponse>, APIError> {
    let debate = schema::tournament_debate::Entity::find_by_id(debate_id).one(&db).await.map_err(
        |_| {
            (hyper::StatusCode::INTERNAL_SERVER_ERROR, "Error getting debate")
        }
    )?;
    let debate = debate.ok_or((hyper::StatusCode::NOT_FOUND, "Debate not found"))?;

    if !check_is_authorized_for_debate_result_submission(&db, &user, debate_id).await? {
        return  Err((hyper::StatusCode::FORBIDDEN, "Not authorized for debate"))?;
    }

    let ballot = DisplayBallot::from_id(debate.ballot_id, &db).await?;

    Ok(Json(GetDebateResponse {
        ballot
    }))
}

async fn submit_ballot(
    State(db): State<DatabaseConnection>,
    Path(debate_id): Path<Uuid>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Json(request): Json<SubmitBallotRequest>,
) -> Result<Json<SubmitBallotResponse>, APIError> {
    if !check_is_authorized_for_debate_result_submission(&db, &user, debate_id).await? {
        return  Err((hyper::StatusCode::FORBIDDEN, "Not authorized for debate"))?;
    }

    let transaction = db.begin().await.map_err(
        |_| {
            (hyper::StatusCode::INTERNAL_SERVER_ERROR, "Error starting transaction")
        }
    )?;

    let tournament_id = schema::tournament_round::Entity::find()
    .inner_join(
        schema::tournament_debate::Entity
    )
    .filter(
        schema::tournament_debate::Column::Uuid.eq(debate_id)
    )
    .one(&transaction).await.map_err(
        |_| {
            (hyper::StatusCode::INTERNAL_SERVER_ERROR, "Error getting tournament id")
        }
    )?.ok_or((hyper::StatusCode::INTERNAL_SERVER_ERROR, "Error getting tournament id"))?.tournament_id;

    let mut ballot = request.ballot;
    let ballot_uuid = Uuid::new_v4();
    ballot.uuid = ballot_uuid;

    let submission_uuid = Uuid::new_v4();
    let submission = DebateBackupBallot {
        uuid: submission_uuid,
        debate_id,
        ballot_id: ballot.uuid,
        timestamp: Utc::now().naive_utc(),
    };

    let group = EntityGroup::new_with_entities(
        vec![
            Entity::Ballot(ballot),
            Entity::DebateBackupBallot(submission)
        ]
    );

    group.save_all_and_log_for_tournament(&transaction, tournament_id).await?;

    transaction.commit().await.map_err(
        |e| {
            tracing::error!("Error committing transaction: {:?}", e);
            (hyper::StatusCode::INTERNAL_SERVER_ERROR, "Error committing transaction")
        }
    )?;

    Ok(Json(SubmitBallotResponse {
        submission_id: submission_uuid,
        ballot_id: ballot_uuid
    }))
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/submission/:submission_id", get(get_ballot_submission)
        )
        .route(
            "/debate/:debate_id/submissions", post(submit_ballot)
        ).route(
            "/debate/:debate_id", get(get_debate)
        )
}