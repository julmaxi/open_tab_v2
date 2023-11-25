use std::{str::FromStr, collections::HashMap};

use axum::{extract::Path, extract::State, Json, Router, routing::{get, post}};
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{domain::{feedback_form::{FeedbackForm, FeedbackSourceRole, FeedbackTargetRole}, entity::LoadEntity, feedback_question::{FeedbackQuestion, QuestionType}, feedback_response::{FeedbackResponseValue, FeedbackResponse}}, prelude::{Participant, Team}, EntityGroup, Entity, EntityGroupTrait};
use sea_orm::{DatabaseConnection, prelude::Uuid, ConnectionTrait, EntityTrait, QueryFilter, RelationTrait, JoinType, QuerySelect, ColumnTrait};
use serde::{Serialize, Deserialize};


use crate::{response::{APIError, handle_error}, state::AppState, auth::ExtractAuthenticatedUser};


#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackFormResponse {
    pub questions: Vec<FeedbackFormQuestion>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackFormQuestion {
    pub uuid: Uuid,
    pub short_name: String,
    pub full_name: String,
    pub description: String,
    pub question_type: QuestionType
}


impl From<FeedbackQuestion> for FeedbackFormQuestion {
    fn from(question: FeedbackQuestion) -> Self {
        Self {
            uuid: question.uuid,
            short_name: question.short_name,
            full_name: question.full_name,
            description: question.description,
            question_type: question.question_config
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool{val: bool},
    Int{val: i32},
    String{val: String}
}

impl From<FeedbackResponseValue> for Value {
    fn from(value: FeedbackResponseValue) -> Self {
        match value {
            FeedbackResponseValue::Bool {val} => Value::Bool{val},
            FeedbackResponseValue::Int {val} => Value::Int{val},
            FeedbackResponseValue::String {val} => Value::String{val}
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackFormSubmissionRequest {
    pub answers: HashMap<Uuid, Value>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackFormSubmissionResponse {
    submission_id: Uuid   
}


async fn get_feedback_form(
    State(db): State<DatabaseConnection>,
    Path((source_role, target_role, _debate_id, _target_id, source_id)): Path<(String, String, Uuid, Uuid, Uuid)>,
    ExtractAuthenticatedUser(_user): ExtractAuthenticatedUser
) -> Result<Json<FeedbackFormResponse>, APIError> {
    let source_role = FeedbackSourceRole::from_str(&source_role).map_err(handle_error)?;
    let target_role = FeedbackTargetRole::from_str(&target_role).map_err(handle_error)?;

    let tournament_id = match source_role {
        FeedbackSourceRole::Chair | FeedbackSourceRole::Wing | FeedbackSourceRole::President | FeedbackSourceRole::NonAligned => {
            Participant::get(&db, source_id).await?.tournament_id
        },
        FeedbackSourceRole::Team => {
            Team::get(&db, source_id).await?.tournament_id
        }
    };

    let questions = get_relevant_questions(&db, tournament_id, source_role, target_role).await?;

    return Ok(Json(
        FeedbackFormResponse {
            questions
        }
    ))
}

async fn get_relevant_questions<C>(db: &C, tournament_id: Uuid, source_role: FeedbackSourceRole, target_role: FeedbackTargetRole) -> Result<Vec<FeedbackFormQuestion>, APIError> where C: sea_orm::ConnectionTrait {
    let relevant_forms = FeedbackForm::get_all_in_tournament(db, tournament_id).await?;
    let relevant_forms = relevant_forms.into_iter().filter(|f| {
        f.is_valid_for_direction(source_role, target_role)
    }).collect_vec();
    let question_ids = relevant_forms.iter().flat_map(|f| f.questions.iter().cloned()).collect_vec();
    let questions = FeedbackQuestion::get_many(db, question_ids).await?;
    let mut question_map = questions.into_iter().map(|q| (q.uuid, q.into())).collect::<std::collections::HashMap<_, FeedbackFormQuestion>>();
    let questions = relevant_forms.iter().flat_map(|f| {
        f.questions.iter().filter_map(|q| {
            question_map.remove(q)
        }).collect_vec()
    }).collect_vec();
    Ok(questions)
}


async fn submit_feedback_form(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path((source_role, target_role, debate_id, target_id, source_id)): Path<(String, String, Uuid, Uuid, Uuid)>,
    Json(submission): Json<FeedbackFormSubmissionRequest>,
) -> Result<Json<FeedbackFormSubmissionResponse>, APIError> {
    let source_role = FeedbackSourceRole::from_str(&source_role).map_err(handle_error)?;
    let target_role = FeedbackTargetRole::from_str(&target_role).map_err(handle_error)?;
    
    let tournament_id = match source_role {
        FeedbackSourceRole::Chair | FeedbackSourceRole::Wing | FeedbackSourceRole::President | FeedbackSourceRole::NonAligned => {
            Participant::get(&db, source_id).await?.tournament_id
        },
        FeedbackSourceRole::Team => {
            Team::get(&db, source_id).await?.tournament_id
        }
    };

    let questions = get_relevant_questions(&db, tournament_id, source_role, target_role).await?;
    let question_map = questions.into_iter().map(|q| (q.uuid, q)).collect::<std::collections::HashMap<_, FeedbackFormQuestion>>();

    let mut response_values = HashMap::new();
    for (key, val) in submission.answers {
        let question = question_map.get(&key).ok_or(APIError::from((StatusCode::BAD_REQUEST, "Invalid question")))?;

        let response_val = match (&question.question_type, &val) {
            (QuestionType::RangeQuestion { config }, Value::Int { val }) => {
                if val < &config.min || val > &config.max {
                    Err(APIError::from((StatusCode::BAD_REQUEST, "Invalid range")))
                }
                else {
                    Ok(FeedbackResponseValue::Int { val: *val })
                }
            },
            (QuestionType::RangeQuestion {..}, _) => {
                Err(APIError::from((StatusCode::BAD_REQUEST, "Invalid value")))
            },
            (QuestionType::TextQuestion, Value::String { val }) => {
                if val.len() > 1024 {
                    Err(APIError::from((StatusCode::BAD_REQUEST, "Text too long")))
                }
                else {
                    Ok(FeedbackResponseValue::String { val: val.clone() })
                }
            },
            (QuestionType::TextQuestion, _) => {
                Err(APIError::from((StatusCode::BAD_REQUEST, "Invalid value")))
            },
            (QuestionType::YesNoQuestion, Value::Bool { val }) => {
                Ok(FeedbackResponseValue::Bool { val: *val })
            },
            (QuestionType::YesNoQuestion, _) => {
                Err(APIError::from((StatusCode::BAD_REQUEST, "Invalid value")))
            },
        }?;
        
        match &response_val {
            FeedbackResponseValue::String { val } if val.len() == 0 => (),
            _ => {response_values.insert(key, response_val);}
        };
    }

    let submission_id = Uuid::new_v4();

    let (source_participant_id, source_team_id) = match source_role {
        FeedbackSourceRole::Chair | FeedbackSourceRole::Wing | FeedbackSourceRole::President | FeedbackSourceRole::NonAligned => {
            let is_authorized = user.check_is_authorized_as_participant(&db, source_id).await?;
            if !is_authorized {
                return Err(APIError::from((StatusCode::FORBIDDEN, "User is not allowed to submit feedback for this participant")))
            }

            (Some(source_id), None)
        },
        FeedbackSourceRole::Team => {
            
            if !user.check_is_authorized_as_member_of_team(&db, source_id).await? {
                return Err(APIError::from((StatusCode::FORBIDDEN, "User is not allowed to submit feedback for this participant")))
            }
            (None, Some(source_id))
        }
    };


    let participant = open_tab_entities::schema::user_participant::Entity::find()
    .join(JoinType::InnerJoin, open_tab_entities::schema::user_participant::Relation::Participant.def())
    .filter(
        open_tab_entities::schema::participant::Column::TournamentId.eq(tournament_id).and(
            open_tab_entities::schema::user_participant::Column::UserId.eq(user.uuid)            
        )
    ).one(&db).await.map_err(handle_error)?;
    
    if participant.is_none() {
        return Err(APIError::from((StatusCode::FORBIDDEN, "User is not a participant in this tournament")))
    }

    let submission = FeedbackResponse {
        uuid: submission_id,
        author_participant_id: participant.unwrap().participant_id,
        target_participant_id: target_id,
        source_team_id,
        source_participant_id,
        source_debate_id: debate_id,
        values: response_values,
    };

    let group = EntityGroup::from(vec![Entity::FeedbackResponse(submission)]);
    group.save_all_and_log_for_tournament(&db, tournament_id).await?;

    return Ok(Json(
        FeedbackFormSubmissionResponse {
            submission_id
        }
    ))
}



pub fn router() -> Router<AppState> {
    Router::new()
    .route("/feedback/:source_role/:target_role/debate/:debate_id/for/:target_id/from/:source_id", get(get_feedback_form))
    .route("/feedback/:source_role/:target_role/debate/:debate_id/for/:target_id/from/:source_id", post(submit_feedback_form))
}