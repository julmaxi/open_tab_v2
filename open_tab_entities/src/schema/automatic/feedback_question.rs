//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.6

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "feedback_question")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub tournament_id: Option<Uuid>,
    pub short_name: String,
    pub full_name: String,
    pub description: String,
    pub question_config: String,
    pub is_confidential: bool,
    pub is_required: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::feedback_form_question::Entity")]
    FeedbackFormQuestion,
    #[sea_orm(has_many = "super::feedback_response_value::Entity")]
    FeedbackResponseValue,
    #[sea_orm(
        belongs_to = "super::tournament::Entity",
        from = "Column::TournamentId",
        to = "super::tournament::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Tournament,
}

impl Related<super::feedback_form_question::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FeedbackFormQuestion.def()
    }
}

impl Related<super::feedback_response_value::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FeedbackResponseValue.def()
    }
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
    }
}

impl Related<super::feedback_form::Entity> for Entity {
    fn to() -> RelationDef {
        super::feedback_form_question::Relation::FeedbackForm.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::feedback_form_question::Relation::FeedbackQuestion
                .def()
                .rev(),
        )
    }
}

impl Related<super::feedback_response::Entity> for Entity {
    fn to() -> RelationDef {
        super::feedback_response_value::Relation::FeedbackResponse.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::feedback_response_value::Relation::FeedbackQuestion
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
