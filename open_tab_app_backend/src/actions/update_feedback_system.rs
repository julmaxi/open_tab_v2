use std::collections::HashMap;

use async_trait::async_trait;
use open_tab_entities::{domain::{self, feedback_form::FeedbackForm}, Entity, EntityGroup, EntityTypeId};
use sea_orm::prelude::Uuid;

use crate::{feedback::QuestionInfo, feedback_forms_view::TemporaryID, views::feedback_forms_view::FeedbackFormInfo};

use super::ActionTrait;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateFeedbackSystemAction {
    tournament_id: Uuid,
    forms: Vec<FeedbackFormInfo>,
    questions: HashMap<TemporaryID, QuestionInfo>
}

#[async_trait]
impl ActionTrait for UpdateFeedbackSystemAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {        
        let existing_forms = domain::feedback_form::FeedbackForm::get_all_in_tournament(db, self.tournament_id).await?;

        let existing_questions = domain::feedback_question::FeedbackQuestion::get_all_in_tournament(db, self.tournament_id).await?;
        
        let new_form_ids_to_uuid = self.forms.iter().filter_map(|x| 
            match &x.uuid {
                TemporaryID::Temporary(v) => Some((v.clone(), Uuid::new_v4())),
                _ => None
            }
        ).collect::<HashMap<_, _>>();

        let new_question_ids_to_uuid = self.questions.iter().filter_map(|(k, v)| 
            match k {
                TemporaryID::Temporary(v) => Some((v.clone(), Uuid::new_v4())),
                _ => None
            }
        ).collect::<HashMap<_, _>>();

        let mut entity_group = EntityGroup::new(self.tournament_id);

        for form in self.forms.iter() {
            let form_id = match &form.uuid {
                TemporaryID::Uuid(v) => v.clone(),
                TemporaryID::Temporary(v) => new_form_ids_to_uuid.get(v).unwrap().clone()
            };

            let form = form.clone();

            let form_entity = FeedbackForm {
                uuid: form_id,
                name: form.name,
                questions: form.questions.iter().map(|x| 
                    match x {
                        TemporaryID::Uuid(v) => v.clone(),
                        TemporaryID::Temporary(v) => new_question_ids_to_uuid.get(v).unwrap().clone()
                    }
                ).collect(),
                visibility: form.visibility,
                tournament_id: Some(self.tournament_id)
            };

            entity_group.add(Entity::FeedbackForm(form_entity));
        }

        for (question_id, question) in self.questions {
            let question_id = match question_id {
                TemporaryID::Uuid(v) => v.clone(),
                TemporaryID::Temporary(v) => new_question_ids_to_uuid.get(&v).unwrap().clone()
            };

            let question_entity = domain::feedback_question::FeedbackQuestion {
                uuid: question_id,
                short_name: question.short_name,
                full_name: question.full_name,
                description: question.description.unwrap_or_default(),
                question_config: question.config.into(),
                tournament_id: Some(self.tournament_id),
                is_confidential: question.is_confidential,
                is_required: question.is_required
            };

            entity_group.add(Entity::FeedbackQuestion(question_entity));
        }

        let deleted_forms = existing_forms.iter().filter(|x| !self.forms.iter().any(|y| 
            match &y.uuid {
                TemporaryID::Uuid(v) => v == &x.uuid,
                _ => false
            }
        )).collect::<Vec<_>>();

        // We do not delete questions here to avoid inadvertent loss of responses.
        // Forms are fine to delete.
        for form in deleted_forms {
            entity_group.delete(EntityTypeId::FeedbackForm, form.uuid);
        }

        Ok(entity_group)
    }
}