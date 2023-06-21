use open_tab_macros::SimpleEntity;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::schema::tournament;

use async_trait::async_trait;
use sea_orm::{prelude::*, ConnectionTrait};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QuestionType {
    RangeQuestion{
        config: RangeQuestionConfig
    },
    TextQuestion,
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
}