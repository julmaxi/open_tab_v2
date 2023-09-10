//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0-rc.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub annoucements_password: Option<String>
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::feedback_form::Entity")]
    FeedbackForm,
    #[sea_orm(has_many = "super::feedback_question::Entity")]
    FeedbackQuestion,
    #[sea_orm(has_many = "super::team::Entity")]
    Team,
    #[sea_orm(has_many = "super::tournament_break::Entity")]
    TournamentBreak,
    #[sea_orm(has_many = "super::tournament_institution::Entity")]
    TournamentInstitution,
    #[sea_orm(has_many = "super::tournament_log::Entity")]
    TournamentLog,
    #[sea_orm(has_many = "super::tournament_round::Entity")]
    TournamentRound,
    #[sea_orm(has_many = "super::tournament_venue::Entity")]
    TournamentVenue,
    #[sea_orm(has_many = "super::user_access_key::Entity")]
    UserAccessKey,
}

impl Related<super::feedback_form::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FeedbackForm.def()
    }
}

impl Related<super::feedback_question::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FeedbackQuestion.def()
    }
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl Related<super::tournament_break::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentBreak.def()
    }
}

impl Related<super::tournament_institution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentInstitution.def()
    }
}

impl Related<super::tournament_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentLog.def()
    }
}

impl Related<super::tournament_round::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentRound.def()
    }
}

impl Related<super::tournament_venue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentVenue.def()
    }
}

impl Related<super::user_access_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAccessKey.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_tournament::Relation::User.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_tournament::Relation::Tournament.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
