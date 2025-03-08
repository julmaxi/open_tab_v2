use std::{str::FromStr, collections::HashMap};

use axum::{extract::{Path, State}, response::{IntoResponse, Response}, routing::{get, post}, Json, Router};
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{derived_models::{compute_question_summary_values, SummaryValue}, domain::{entity::LoadEntity, feedback_form::{FeedbackForm, FeedbackSourceRole, FeedbackTargetRole}, feedback_question::{FeedbackQuestion, QuestionType, DEFAULT_TEXT_MAX_LENGTH}, feedback_response::{FeedbackResponse, FeedbackResponseValue}}, prelude::{Participant, Team}, schema, Entity, EntityGroup};
use rand::{thread_rng, seq::SliceRandom};
use sea_orm::{DatabaseConnection, prelude::Uuid, EntityTrait, QueryFilter, RelationTrait, JoinType, QuerySelect, ColumnTrait, TransactionTrait, QueryOrder};
use serde::{Serialize, Deserialize};


use crate::{auth::ExtractAuthenticatedUser, response::APIError, state::AppState};


#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackFormResponse {
    pub questions: Vec<FeedbackFormQuestion>,
    pub target_name: String,
    pub target_round_index: i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackFormQuestion {
    pub uuid: Uuid,
    pub short_name: String,
    pub full_name: String,
    pub description: String,
    pub question_type: QuestionType,
    pub is_required: bool
}


impl From<FeedbackQuestion> for FeedbackFormQuestion {
    fn from(question: FeedbackQuestion) -> Self {
        Self {
            uuid: question.uuid,
            short_name: question.short_name,
            full_name: question.full_name,
            description: question.description,
            question_type: question.question_config,
            is_required: question.is_required
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

#[derive(Debug, Serialize)]
pub struct FeedbackFormSubmissionResponse {
    submission_id: Option<Uuid>,
    values: HashMap<Uuid, serde_json::Value>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    validation_errors: HashMap<Uuid, String>
}

impl IntoResponse for FeedbackFormSubmissionResponse {
    fn into_response(self) -> Response {
        let mut res = serde_json::to_string(&self).unwrap().into_response();
        if self.validation_errors.len() > 0 {
            *res.status_mut() = StatusCode::BAD_REQUEST;
        }
        res
    }
}

fn response_value_to_serde_value(value: &FeedbackResponseValue) -> serde_json::Value {
    match value {
        FeedbackResponseValue::Bool {val} => serde_json::Value::Bool(*val),
        FeedbackResponseValue::Int {val} => serde_json::Value::Number(serde_json::Number::from(*val)),
        FeedbackResponseValue::String {val} => serde_json::Value::String(val.clone())
    }
}

async fn get_feedback_form(
    State(db): State<DatabaseConnection>,
    Path((source_role, target_role, debate_id, target_id, source_id)): Path<(String, String, Uuid, Uuid, Uuid)>,
    ExtractAuthenticatedUser(_user): ExtractAuthenticatedUser
) -> Result<Json<FeedbackFormResponse>, APIError> {
    let source_role = FeedbackSourceRole::from_str(&source_role)?;
    let target_role = FeedbackTargetRole::from_str(&target_role)?;

    let tournament_id = match source_role {
        FeedbackSourceRole::Chair | FeedbackSourceRole::Wing | FeedbackSourceRole::President | FeedbackSourceRole::NonAligned => {
            Participant::get(&db, source_id).await?.tournament_id
        },
        FeedbackSourceRole::Team => {
            Team::get(&db, source_id).await?.tournament_id
        }
    };

    let target_participant = schema::participant::Entity::find_by_id(target_id).one(&db).await?;
    if target_participant.is_none() {
        return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Invalid participant"))
    }
    let target_participant = target_participant.unwrap();
    
    let target_round = schema::tournament_round::Entity::find().inner_join(
        schema::tournament_debate::Entity
    ).filter(
        schema::tournament_debate::Column::Uuid.eq(debate_id)
    ).one(&db).await?;

    if target_round.is_none() {
        return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Invalid debate"))
    }
    let target_round = target_round.unwrap();

    let questions = get_relevant_questions(&db, tournament_id, source_role, target_role).await?;

    return Ok(Json(
        FeedbackFormResponse {
            questions,
            target_name: target_participant.name,
            target_round_index: target_round.index
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
) -> Result<FeedbackFormSubmissionResponse, APIError> {
    let source_role = FeedbackSourceRole::from_str(&source_role)?;
    let target_role = FeedbackTargetRole::from_str(&target_role)?;

    let db = db.begin().await?;
    
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
    let mut errors = HashMap::new();
    for (key, val) in submission.answers {
        let question: &FeedbackFormQuestion = question_map.get(&key).ok_or_else(|| APIError::new_with_status(StatusCode::BAD_REQUEST, format!("Invalid question {}", key)))?;

        let response_val = match (&question.question_type, &val) {
            (QuestionType::RangeQuestion { config }, Value::Int { val }) => {
                if val < &config.min || val > &config.max {
                    Err("Invalid range")
                }
                else {
                    Ok(FeedbackResponseValue::Int { val: *val })
                }
            },
            (QuestionType::RangeQuestion {..}, _) => {
                Err("Invalid value")
            },
            (QuestionType::TextQuestion { .. }, Value::String { val }) => {
                Ok(FeedbackResponseValue::String { val: val.clone() })
            },
            (QuestionType::TextQuestion { .. }, _) => {
                Err("Invalid value")
            },
            (QuestionType::YesNoQuestion, Value::Bool { val }) => {
                Ok(FeedbackResponseValue::Bool { val: *val })
            },
            (QuestionType::YesNoQuestion, _) => {
                Err("Invalid value")
            },
        };
        
        match response_val {
            Ok(val) => {
                match val {
                    FeedbackResponseValue::String { val } if val.len() == 0 => (),
                    _ => {response_values.insert(key, val);}        
                }
            },
            Err(text) => {
                errors.insert(key, text.to_owned());
            }
        };
    }

    for (key, question) in question_map.iter() {
        let value = response_values.get(key);
        if let Some(value) = value {
            if let FeedbackResponseValue::String { val } = value {
                if let QuestionType::TextQuestion { config } = &question.question_type {
                    if val.len() > (config.max_length as usize) {
                        errors.insert(*key, "Text too long".into());
                    }
                }
            }
        }
        else {
            if question.is_required {
                errors.insert(*key, "Required".into());
            }
        }
    }

    if errors.len() > 0 {
        return Ok(
            FeedbackFormSubmissionResponse {
                submission_id: None,
                values: response_values.into_iter().map(|(k, v)| (k, response_value_to_serde_value(&v))).collect(),
                validation_errors: errors
            }
        )
    }

    let submission_id = Uuid::new_v4();

    let (source_participant_id, source_team_id) = match source_role {
        FeedbackSourceRole::Chair | FeedbackSourceRole::Wing | FeedbackSourceRole::President | FeedbackSourceRole::NonAligned => {
            let is_authorized = user.check_is_authorized_as_participant(&db, source_id).await?  || user.check_is_authorized_for_tournament_administration(&db, tournament_id).await?;
            if !is_authorized {
                return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not allowed to submit feedback for this participant"))
            }

            (Some(source_id), None)
        },
        FeedbackSourceRole::Team => {
            
            if !(user.check_is_authorized_as_member_of_team(&db, source_id).await? || user.check_is_authorized_for_tournament_administration(&db, tournament_id).await?) {
                return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not allowed to submit feedback for this participant"))
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
    ).one(&db).await?;
    
    if participant.is_none() {
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not a participant in this tournament"))
    }

    let submission = FeedbackResponse {
        uuid: submission_id,
        author_participant_id: participant.unwrap().participant_id,
        target_participant_id: target_id,
        source_team_id,
        source_participant_id,
        source_debate_id: debate_id,
        values: response_values.clone(),
    };

    let group = EntityGroup::new_from_entities(
        tournament_id,
        vec![Entity::FeedbackResponse(submission)]
    );
    group.save_all_and_log(&db).await?;

    db.commit().await?;

    return Ok(
        FeedbackFormSubmissionResponse {
            submission_id: Some(submission_id),
            values: response_values.into_iter().map(|(k, v)| (k, response_value_to_serde_value(&v))).collect(),
            validation_errors: HashMap::new()
        }
    )
}

#[derive(Debug, Serialize, Deserialize)]
struct ParticipantFeedbackSummary {
    summary_values: Vec<ParticipantFeedbackSummaryValue>,
    individual_values: Vec<ParticipantFeedbackIndividualValueList>
}

#[derive(Debug, Serialize, Deserialize)]
struct ParticipantFeedbackSummaryValue {
    question_name: String,
    question_uuid: Uuid,
    #[serde(flatten)]
    value: SummaryValue
}

#[derive(Debug, Serialize, Deserialize)]
struct ParticipantFeedbackIndividualValueList {
    question_name: String,
    question_uuid: Uuid,
    values: Vec<FeedbackResponseValue>
}


async fn get_participant_feedback_summary(State(db): State<DatabaseConnection>, Path(participant_id): Path<Uuid>, ExtractAuthenticatedUser(user): ExtractAuthenticatedUser) -> Result<Json<ParticipantFeedbackSummary>, APIError> {
    if !user.check_is_authorized_as_participant(&db, participant_id).await? {
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not allowed to view feedback for this participant"))
    }
    let now = chrono::Utc::now().naive_utc();

    let db = db.begin().await?;
    let relevant_answer_values = schema::feedback_response_value::Entity::find().inner_join(
        schema::feedback_response::Entity
    )
    .join(JoinType::InnerJoin, schema::feedback_response::Relation::TournamentDebate.def())
    .join(JoinType::InnerJoin, schema::tournament_debate::Relation::TournamentRound.def())
    .filter(
        schema::tournament_round::Column::FeedbackReleaseTime.lte(now).and(
            schema::feedback_response::Column::TargetParticipantId.eq(participant_id)
        )
    )
    .all(&db).await?;

    let question_ids = relevant_answer_values.iter().map(|v| v.question_id).collect_vec();

    let relevant_questions = schema::feedback_question::Entity::find().filter(
        schema::feedback_question::Column::Uuid.is_in(question_ids).and(
            schema::feedback_question::Column::IsConfidential.eq(false)
        )
    ).order_by_asc(schema::feedback_question::Column::FullName).all(&db).await?.into_iter().map(FeedbackQuestion::from_model).collect_vec();
    db.rollback().await?;

    let answers_by_question_id : Result<Vec<(Uuid, FeedbackResponseValue)>, anyhow::Error> = relevant_answer_values.into_iter().map(|a| Ok((a.question_id, FeedbackResponseValue::try_from(a)?))).collect();
    let answers_by_question_id = answers_by_question_id?.into_iter().into_group_map();

    let questions_by_id = relevant_questions.iter().map(|q| (q.uuid, q.clone())).collect::<HashMap<_, _>>();
    let summary_values = compute_question_summary_values(&answers_by_question_id, &questions_by_id);

    let summary_values = relevant_questions.iter().filter_map(
        |q| {
            let summary_value = summary_values.get(&q.uuid).unwrap_or(&SummaryValue::Unavailable);
            match summary_value {
                SummaryValue::Unavailable => None,
                v => Some(ParticipantFeedbackSummaryValue {
                    question_name: q.full_name.clone(),
                    question_uuid: q.uuid,
                    value: v.clone()
            })
        }
    }
    ).collect();

    let individual_values = relevant_questions.iter().filter_map(
        |q| {
            match q.question_config {
                QuestionType::TextQuestion { .. } => {
                    let mut values = answers_by_question_id.get(&q.uuid).unwrap_or(&vec![]).clone();
                    values.shuffle(&mut thread_rng());
                    Some(ParticipantFeedbackIndividualValueList {
                        question_name: q.full_name.clone(),
                        question_uuid: q.uuid,
                        values
                    })
                },
                _ => None
            }
        }
    ).collect();

    Ok(
        Json(
            ParticipantFeedbackSummary {
                summary_values,
                individual_values
            }
        )
    )
}


pub fn router() -> Router<AppState> {
    Router::new()
    .route("/feedback/:source_role/:target_role/debate/:debate_id/for/:target_id/from/:source_id", get(get_feedback_form))
    .route("/feedback/:source_role/:target_role/debate/:debate_id/for/:target_id/from/:source_id", post(submit_feedback_form))
    .route("/participant/:participant_id/feedback", get(get_participant_feedback_summary))
}