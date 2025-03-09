use std::{collections::HashMap, ops::{Add, AddAssign}};

use itertools::Itertools;
use open_tab_entities::{domain::{self, entity::LoadEntity, feedback_form::FeedbackFormVisibility, tournament_plan_node::PlanNodeType}, info::TournamentParticipantsInfo, schema, EntityGroup, EntityTypeId};
use sea_orm::{prelude::Uuid, EntityOrSelect, EntityTrait, QueryFilter, QuerySelect, ColumnTrait};
use serde::{Deserialize, Serialize};

use crate::{feedback::QuestionInfo, tournament_tree_view::get_round_names, LoadedView};



pub struct LoadedFeedbackFormsView {
    tournament_uuid: Uuid,
    view: FeedbackFormsView
}

impl LoadedFeedbackFormsView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            Self {
                tournament_uuid,
                view: FeedbackFormsView::load(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait::async_trait]
impl LoadedView for LoadedFeedbackFormsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_types(
            vec![
                EntityTypeId::FeedbackForm,
                EntityTypeId::FeedbackQuestion,
            ]
        ) {
            self.view = FeedbackFormsView::load(db, self.tournament_uuid).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum TemporaryID {
    Uuid(Uuid),
    Temporary(String)
}

impl From<Uuid> for TemporaryID {
    fn from(uuid: Uuid) -> Self {
        TemporaryID::Uuid(uuid)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackFormInfo {
    pub uuid: TemporaryID,
    pub name: String,
    pub questions: Vec<TemporaryID>,
    pub visibility: FeedbackFormVisibility
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackFormsView {
    forms: Vec<FeedbackFormInfo>,
    questions: HashMap<TemporaryID, QuestionInfo>
}

impl FeedbackFormsView {
    async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let feedback_forms = domain::feedback_form::FeedbackForm::get_all_in_tournament(db, tournament_id).await?;

        let questions : HashMap<TemporaryID, QuestionInfo> = domain::feedback_question::FeedbackQuestion::get_all_in_tournament(db, tournament_id).await?.into_iter().map(|x| (x.uuid.clone().into(), x.into())).collect::<HashMap<_, _>>();

        let feedback_forms = feedback_forms.into_iter().map(|form| {
            FeedbackFormInfo {
                uuid: form.uuid.clone().into(),
                name: form.name,
                questions: form.questions.iter().map(|x| x.clone().into()).collect(),
                visibility: form.visibility
            }
        }).collect::<Vec<_>>();

        Ok(FeedbackFormsView {
            forms: feedback_forms,
            questions
        })
    }
}