use std::{collections::HashMap, error::Error};

use itertools::Itertools;
use open_tab_entities::{domain::{feedback_question::{FeedbackQuestion, QuestionType}, feedback_response::FeedbackResponseValue}, EntityGroup};
use sea_orm::{prelude::*, schema, QueryOrder, JoinType, QuerySelect, DatabaseTransaction};
use serde::{Serialize, Deserialize};

use crate::{View, LoadedView};



pub struct LoadedFeedbackOverviewView {
    pub tournament_id: Uuid,
    view: FeedbackOverviewView
}


impl LoadedFeedbackOverviewView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<LoadedFeedbackOverviewView, Box<dyn Error>> where C: ConnectionTrait {
        Ok(
            LoadedFeedbackOverviewView {
                tournament_id: tournament_uuid,
                view: FeedbackOverviewView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedFeedbackOverviewView {
    async fn update_and_get_changes(&mut self, db: &DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.participants.len() > 0 || changes.feedback_responses.len() > 0 || changes.feedback_forms.len() > 0 || changes.feedback_questions.len() > 0 {
            self.view = FeedbackOverviewView::load_from_tournament(db, self.tournament_id).await?;

            let mut out: HashMap<String, Json> = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }
    async fn view_string(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.view)?)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackOverviewView {
    pub participant_entries: Vec<ParticipantEntry>,
    pub summary_columns: Vec<SummaryColumn>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryColumn {
    pub question_id: Uuid,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum SummaryValue {
    Average{avg: f32},
    Percentage{percentage: f32},
    Unavailable
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantEntry {
    pub participant_id: Uuid,
    pub participant_name: String,
    pub score_summaries: HashMap<Uuid, SummaryValue>
}


impl FeedbackOverviewView {
    pub async fn load_from_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        let adjudicators = open_tab_entities::schema::adjudicator::Entity::find()
            .find_also_related(open_tab_entities::schema::participant::Entity)
            .filter(open_tab_entities::schema::participant::Column::TournamentId.eq(tournament_id))
            .order_by_asc(open_tab_entities::schema::participant::Column::Name)
            .all(db).await?;
        // Unwrap uaranteed by db constraints
        let adjudicator_names = adjudicators.iter().map(|(a, p)| (a.uuid, p.clone().unwrap().name.clone())).collect::<HashMap<_, _>>();

        let questions = open_tab_entities::schema::feedback_question::Entity::find()
            .join(JoinType::InnerJoin, open_tab_entities::schema::feedback_form_question::Relation::FeedbackQuestion.def().rev())
            .join(JoinType::InnerJoin, open_tab_entities::schema::feedback_form_question::Relation::FeedbackForm.def())
            .filter(open_tab_entities::schema::feedback_form::Column::TournamentId.eq(tournament_id))
            .order_by_asc(open_tab_entities::schema::feedback_form_question::Column::Index)
            .distinct()
            .all(db).await?;

        let questions = questions.into_iter().map(open_tab_entities::domain::feedback_question::FeedbackQuestion::from_model).collect::<Vec<_>>();

        let summary_columns = questions.iter().filter(
            |q: &&FeedbackQuestion| match &q.question_config {
                QuestionType::TextQuestion => false,
                _ => true
            }
        ).map(|q| SummaryColumn {
            question_id: q.uuid,
            title: q.short_name.clone()
        }).collect();

        let questions_by_id = questions.into_iter().map(|q| (q.uuid, q)).collect::<HashMap<_, _>>();

        let all_values = open_tab_entities::schema::feedback_response_value::Entity::find()
        .find_also_related(open_tab_entities::schema::feedback_response::Entity)
        .join(JoinType::InnerJoin, open_tab_entities::schema::feedback_response::Relation::TournamentDebate.def())
        .filter(open_tab_entities::schema::feedback_response::Column::TargetParticipantId.is_in(adjudicators.iter().map(|(a, _)| a.uuid)))
        .all(db).await?;

        //TODO: This would be much better as a query, but sea orm is a bit finicky.

        let mut participant_values : HashMap<Uuid, HashMap<Uuid, Vec<FeedbackResponseValue>>> = HashMap::from_iter(
            adjudicators.iter().map(|(adj, _)| (
                adj.uuid, questions_by_id.values().map(|q| (q.uuid, Vec::new())).collect::<HashMap<_, Vec<FeedbackResponseValue>>>()
            ))
        );

        for (value, response) in all_values.into_iter() {
            let response = response.unwrap(); // Guaranteed by db constraints
            let question = questions_by_id.get(&value.question_id).unwrap(); // Guaranteed by db constraints
            participant_values.get_mut(&response.target_participant_id).map(|e| {
                if let Some(val) = question.extract_value_from_response_value_model(&value) {
                    e.get_mut(
                        &value.question_id,
                    ).map(
                        |v| v.push(val)
                    );
                };
            });
        }

        let participant_entries = participant_values.into_iter().map(
            |(participant_id, question_values)| {
                let averages = question_values.into_iter().filter_map(
                    |(question_id, vals)| {
                        //FIXME: This will give unexpected results if question type changes
                        //since we count both old and new value
                        let n_vals = vals.len();
                        let n_vals_f32 = n_vals as f32;
                        let question = questions_by_id.get(&question_id).unwrap(); // Guaranteed by db constraints
                        let summary_val = match &question.question_config {
                            open_tab_entities::domain::feedback_question::QuestionType::RangeQuestion { .. } => Some(SummaryValue::Average{avg: vals.into_iter().filter_map(
                                |v| match v {
                                    open_tab_entities::domain::feedback_response::FeedbackResponseValue::Int { val } => Some(val as f32),
                                    _ => None
                                }
                            ).sum()}),
                            open_tab_entities::domain::feedback_question::QuestionType::TextQuestion => None,
                            open_tab_entities::domain::feedback_question::QuestionType::YesNoQuestion => {
                                let n_yes = vals.into_iter().filter_map(
                                    |v| match v {
                                        open_tab_entities::domain::feedback_response::FeedbackResponseValue::Bool { val } => Some(val),
                                        _ => None
                                    }
                                ).filter(|v| *v).count() as f32;
                                Some(SummaryValue::Percentage{percentage: n_yes / n_vals_f32})
                            },
                        };

                        if let Some(val) = summary_val {
                            if n_vals == 0 {
                                Some((question_id, SummaryValue::Unavailable))
                            }    
                            else {
                                Some((question_id, val))
                            }
                        } else {
                            None
                        }
                    }
                ).collect::<HashMap<_, _>>();

                let name = adjudicator_names.get(&participant_id).unwrap(); // Guaranteed by db constraints
                ParticipantEntry {
                    participant_id,
                    participant_name: name.clone(),
                    score_summaries: averages
                }
            }
        ).collect_vec();

        Ok(FeedbackOverviewView { participant_entries, summary_columns })
    }
}
