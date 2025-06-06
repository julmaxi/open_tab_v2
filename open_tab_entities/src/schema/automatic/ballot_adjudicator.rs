//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "ballot_adjudicator")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub ballot_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub adjudicator_id: Uuid,
    pub position: i32,
    pub role: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::adjudicator::Entity",
        from = "Column::AdjudicatorId",
        to = "super::adjudicator::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Adjudicator,
    #[sea_orm(has_many = "super::adjudicator_speech_score::Entity")]
    AdjudicatorSpeechScore,
    #[sea_orm(has_many = "super::adjudicator_team_score::Entity")]
    AdjudicatorTeamScore,
    #[sea_orm(
        belongs_to = "super::ballot::Entity",
        from = "Column::BallotId",
        to = "super::ballot::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Ballot,
}

impl Related<super::adjudicator::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Adjudicator.def()
    }
}

impl Related<super::adjudicator_speech_score::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdjudicatorSpeechScore.def()
    }
}

impl Related<super::adjudicator_team_score::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdjudicatorTeamScore.def()
    }
}

impl Related<super::ballot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ballot.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
