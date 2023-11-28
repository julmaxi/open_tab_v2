use std::fs::File;

use open_tab_entities::{schema, EntityType};
use open_tab_entities::{EntityGroup, EntityGroupTrait, Entity};
use sea_orm::prelude::*;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;
use crate::feedback::FormTemplate;



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFeedbackSystemAction {
    pub tournament_uuid: Uuid,
    pub template_path: String
}


#[async_trait]
impl ActionTrait for ImportFeedbackSystemAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new();

        let reader = File::open(self.template_path)?;

        let existing_forms = schema::feedback_form::Entity::find().filter(schema::feedback_form::Column::TournamentId.eq(self.tournament_uuid)).all(_db).await?;
        let existing_questions = schema::feedback_question::Entity::find().filter(schema::feedback_question::Column::TournamentId.eq(self.tournament_uuid)).all(_db).await?;

        for form in existing_forms {
            g.delete(EntityType::FeedbackForm, form.uuid);
        }

        for question in existing_questions {
            g.delete(EntityType::FeedbackQuestion, question.uuid);
        }

        let result : FormTemplate = FormTemplate::from_reader(reader)?;

        let (forms, questions) = result.into_forms_and_questions_for_tournament(
            self.tournament_uuid
        )?;

        for form in forms {
            g.add(
                Entity::FeedbackForm(form)
            );
        }

        for question in questions {
            g.add(
                Entity::FeedbackQuestion(question)
            );
        }

        Ok(
            g
        )
    }
}