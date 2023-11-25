use std::collections::HashMap;

use async_trait::async_trait;
use itertools::Itertools;
use sea_orm::{ConnectionTrait, EntityTrait, ColumnTrait, QueryFilter, ActiveValue, ActiveModelTrait};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::{schema, utilities::BatchLoad};

use super::{entity::LoadEntity, TournamentEntity};



#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackResponse {
    pub uuid: Uuid,
    pub author_participant_id: Uuid,
    pub target_participant_id: Uuid,
    pub source_team_id: Option<Uuid>,
    pub source_participant_id: Option<Uuid>,
    pub source_debate_id: Uuid,

    pub values: HashMap<Uuid, FeedbackResponseValue>,
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum FeedbackResponseValue {
    Bool {val: bool},
    Int {val: i32},
    String {val: String},
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
enum LoadFeedbackError {
    #[error("Feedback response {0} has no values")]
    NoValues(Uuid),
    #[error("Feedback response {0} has multiple values")]
    MultipleValues(Uuid, Uuid),
}

#[async_trait]
impl TournamentEntity for FeedbackResponse {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        let existing_response = if guarantee_insert {
            None
        } else {
            schema::feedback_response::Entity::find_by_id(self.uuid.clone()).one(db).await?
        };

        let mut model = schema::feedback_response::ActiveModel {
            uuid: ActiveValue::Set(self.uuid.clone()),
            author_participant_id: ActiveValue::Set(self.author_participant_id.clone()),
            target_participant_id: ActiveValue::Set(self.target_participant_id.clone()),
            source_team_id: ActiveValue::Set(self.source_team_id.clone()),
            source_participant_id: ActiveValue::Set(self.source_participant_id.clone()),
            source_debate_id: ActiveValue::Set(self.source_debate_id.clone()),
            ..Default::default()
        };

        if let Some(existing_response) = &existing_response {
            model.uuid = ActiveValue::Unchanged(existing_response.uuid.clone());
            model.update(db).await?;
        } else {
            model.insert(db).await?;
        }

        let existing_values_by_question_id = if existing_response.is_some() {
            schema::feedback_response_value::Entity::find()
                .filter(schema::feedback_response_value::Column::ResponseId.eq(self.uuid.clone()))
                .all(db).await?.into_iter().map(
                    |v| (v.question_id.clone(), v)
                ).collect::<HashMap<Uuid, schema::feedback_response_value::Model>>()
        }
        else {
            HashMap::new()
        };

        for (question_id, value) in self.values.iter() {
            let existing_value = existing_values_by_question_id.get(question_id);

            let mut response_value_model = schema::feedback_response_value::ActiveModel {
                response_id: ActiveValue::Set(self.uuid.clone()),
                question_id: ActiveValue::Set(question_id.clone()),
                bool_value: ActiveValue::Set(None),
                int_value: ActiveValue::Set(None),
                string_value: ActiveValue::Set(None),
                ..Default::default()
            };

            match value {
                FeedbackResponseValue::Bool {val} => {
                    response_value_model.bool_value = ActiveValue::Set(Some(*val));
                },
                FeedbackResponseValue::Int {val} => {
                    response_value_model.int_value = ActiveValue::Set(Some(*val));
                },
                FeedbackResponseValue::String {val} => {
                    response_value_model.string_value = ActiveValue::Set(Some(val.clone()));
                },
            }

            if let Some(_) = existing_value {
                response_value_model.response_id = ActiveValue::Unchanged(self.uuid.clone());
                response_value_model.question_id = ActiveValue::Unchanged(*question_id);
                response_value_model.update(db).await?;
            } else {
                response_value_model.insert(db).await?;
            }
        }

        Ok(())
    }

    async fn get_many_tournaments<C>(db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let form_tournament_ids = schema::tournament_round::Entity::find()
            .left_join(schema::tournament_debate::Entity)
            .filter(schema::tournament_debate::Column::Uuid.is_in(entities.iter().map(|x| x.source_debate_id.clone()).collect::<Vec<Uuid>>()))
            .all(db).await?.into_iter().map(|x| (x.uuid, Some(x.tournament_id))).collect::<HashMap<Uuid, Option<Uuid>>>();
        
        entities.into_iter().map(|x| Ok(form_tournament_ids.get(&x.uuid).cloned().flatten())).collect()
    }

    async fn delete_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        schema::feedback_response::Entity::delete_many().filter(
            schema::feedback_response::Column::Uuid.is_in(uuids)
        ).exec(db).await?;
        Ok(())
    }

}

#[async_trait]
impl LoadEntity for FeedbackResponse {
    async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Self>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let responses = schema::feedback_response::Entity::batch_load::<_, Uuid>(db, uuids.clone()).await?;
        
        let mut response_values = schema::feedback_response_value::Entity::find()
            .filter(schema::feedback_response_value::Column::ResponseId.is_in(responses.iter().filter_map(|x| x.clone().map(|x| x.uuid.clone())).collect::<Vec<Uuid>>()))
            .all(db).await?.into_iter().into_grouping_map_by(|e| e.response_id).collect::<Vec<_>>();

        let vals : Result<Vec<_>, _> = responses.into_iter().map(
            |f| {
                if let Some(f) = f {
                    let response_values = response_values.remove(&f.uuid);
                    Ok(Some(FeedbackResponse::from_rows(f, response_values.unwrap_or_else(Vec::new))?))                  
                }
                else {
                    Ok(None)
                }
            }
        ).collect();

        vals
    }
}

impl FeedbackResponse {
    fn from_rows(response: schema::feedback_response::Model, values: Vec<schema::feedback_response_value::Model>) -> Result<Self, anyhow::Error> {
        let values : Result<HashMap<_, _>, _> = values.into_iter().map(
            |v| {
                Ok((v.question_id, match (
                        v.bool_value, v.int_value, v.string_value
                    ) {
                        (Some(val), None, None) => FeedbackResponseValue::Bool {val},
                        (None, Some(val), None) => FeedbackResponseValue::Int {val},
                        (None, None, Some(val)) => FeedbackResponseValue::String {val},
                        (None, None, None) => return Err(Box::new(LoadFeedbackError::NoValues(v.response_id))),
                        _ => return Err(Box::new(LoadFeedbackError::MultipleValues(v.response_id, v.question_id))),
                    }
                ))
            }
        ).collect();

        let values = values?;

        Ok(FeedbackResponse {
            uuid: response.uuid,
            author_participant_id: response.author_participant_id,
            target_participant_id: response.target_participant_id,
            source_team_id: response.source_team_id,
            source_participant_id: response.source_participant_id,
            source_debate_id: response.source_debate_id,
            values,
        })
    }

    pub async fn get_all_for_target_participant<C>(db: &C, target_id: Uuid) -> Result<Vec<Self>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let responses = schema::feedback_response::Entity::find()
            .find_with_related(schema::feedback_response_value::Entity)
            .filter(schema::feedback_response::Column::TargetParticipantId.eq(target_id))
            .all(db).await?;
        
        let vals : Result<Vec<_>, _> = responses.into_iter().map(
            |(response, response_values)| {
                Ok(FeedbackResponse::from_rows(response, response_values)?)
            }
        ).collect();

        vals
    }
}