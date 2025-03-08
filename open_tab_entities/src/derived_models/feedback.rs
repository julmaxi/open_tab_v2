use serde::{Serialize, Deserialize};

use sea_orm::entity::prelude::Uuid;
use std::collections::HashMap;
use crate::domain::feedback_question::FeedbackQuestion;
use crate::domain::feedback_response::FeedbackResponseValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum SummaryValue {
    Average{avg: f32},
    Percentage{percentage: f32},
    Unavailable
}

fn safe_avg<I>(iter: I) -> Option<f32> where I: Iterator<Item=f32> {
    let mut sum = 0.0;
    let mut count = 0;
    for i in iter {
        sum += i;
        count += 1;
    }

    if count > 0 {
        Some(sum / count as f32)
    }
    else {
        None
    }
}


pub fn compute_question_summary_values(question_values: &HashMap<Uuid, Vec<FeedbackResponseValue>>, questions_by_id: &HashMap<Uuid, FeedbackQuestion>) -> HashMap<Uuid, SummaryValue> {
    let averages = question_values.into_iter().filter_map(
        |(question_id, vals)| {
            //FIXME: This will give unexpected results if question type changes
            //since we count both old and new value
            let n_vals = vals.len();
            let n_vals_f32 = n_vals as f32;
            let question = questions_by_id.get(&question_id);
            if question.is_none() {
                return None;
            }
            let question = question.unwrap();
            let summary_val = match &question.question_config {
                crate::domain::feedback_question::QuestionType::RangeQuestion { .. } => Some(SummaryValue::Average{avg: safe_avg(vals.into_iter().filter_map(
                    |v| match v {
                        crate::domain::feedback_response::FeedbackResponseValue::Int { val } => Some(*val as f32),
                        _ => None
                    }
                )).unwrap_or(0.0)}),
                crate::domain::feedback_question::QuestionType::TextQuestion { .. } => None,
                crate::domain::feedback_question::QuestionType::YesNoQuestion => {
                    let n_yes = vals.into_iter().filter_map(
                        |v| match v {
                            crate::domain::feedback_response::FeedbackResponseValue::Bool { val } => Some(val),
                            _ => None
                        }
                    ).filter(|v| **v).count() as f32;
                    Some(SummaryValue::Percentage{percentage: n_yes / n_vals_f32})
                },
            };

            if let Some(val) = summary_val {
                if n_vals == 0 {
                    Some((*question_id, SummaryValue::Unavailable))
                }    
                else {
                    Some((*question_id, val))
                }
            } else {
                None
            }
        }
    ).collect::<HashMap<_, _>>();
    averages
}
