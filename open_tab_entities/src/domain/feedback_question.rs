use open_tab_macros::SimpleEntity;
use serde::{Serialize, Deserialize};
use uuid::Uuid;



use async_trait::async_trait;
use sea_orm::prelude::*;


pub const DEFAULT_TEXT_MAX_LENGTH : u32 = 2048;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QuestionType {
    RangeQuestion {
        config: RangeQuestionConfig
    },
    TextQuestion {
        #[serde(default)]
        config: TextQuestionConfig
    },
    YesNoQuestion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RangeQuestionConfig {
    pub min: i32,
    pub max: i32,
    pub orientation: RangeQuestionOrientation,
    pub labels: Vec<(i32, String)>,
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextQuestionConfig {
    pub max_length: u32,
}

impl Default for TextQuestionConfig {
    fn default() -> Self {
        TextQuestionConfig {
            max_length: DEFAULT_TEXT_MAX_LENGTH, // Default max_length value
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RangeQuestionOrientation {
    HighIsGood,
    LowIsGood,
    MeanIsGood
}

impl Default for RangeQuestionOrientation {
    fn default() -> Self {
        RangeQuestionOrientation::HighIsGood
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, SimpleEntity)]
#[module_path="crate::schema::feedback_question"]
#[tournament_id="tournament_id"]
pub struct FeedbackQuestion {
    pub uuid: Uuid,
    pub short_name: String,
    pub full_name: String,
    pub description: String,
    #[serialize]
    pub question_config: QuestionType,

    pub tournament_id: Option<Uuid>,

    pub is_confidential: bool,

    pub is_required: bool,
}

impl FeedbackQuestion {
    pub fn extract_value_from_response_value_model(&self, response: &crate::schema::feedback_response_value::Model) -> Option<crate::domain::feedback_response::FeedbackResponseValue> {
        match &self.question_config {
            QuestionType::RangeQuestion { .. } => {
                response.int_value.clone().map(|val| {
                    crate::domain::feedback_response::FeedbackResponseValue::Int{val}
                })
            },
            QuestionType::TextQuestion { .. } => {
                response.string_value.clone().map(|val| {
                    crate::domain::feedback_response::FeedbackResponseValue::String{val}
                })
            },
            QuestionType::YesNoQuestion => {
                response.bool_value.clone().map(|val| {
                    crate::domain::feedback_response::FeedbackResponseValue::Bool{val}
                })
            }
        }
    }
}