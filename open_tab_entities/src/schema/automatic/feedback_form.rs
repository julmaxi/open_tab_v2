//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "feedback_form")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub tournament_id: Option<Uuid>,
    pub name: String,
    pub show_chairs_for_presidents: bool,
    pub show_chairs_for_wings: bool,
    pub show_wings_for_chairs: bool,
    pub show_wings_for_presidents: bool,
    pub show_wings_for_wings: bool,
    pub show_presidents_for_chairs: bool,
    pub show_presidents_for_wings: bool,
    pub show_teams_for_chairs: bool,
    pub show_teams_for_presidents: bool,
    pub show_teams_for_wings: bool,
    pub show_non_aligned_for_chairs: bool,
    pub show_non_aligned_for_presidents: bool,
    pub show_non_aligned_for_wings: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::feedback_form_question::Entity")]
    FeedbackFormQuestion,
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

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
    }
}

impl Related<super::feedback_question::Entity> for Entity {
    fn to() -> RelationDef {
        super::feedback_form_question::Relation::FeedbackQuestion.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::feedback_form_question::Relation::FeedbackForm
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
