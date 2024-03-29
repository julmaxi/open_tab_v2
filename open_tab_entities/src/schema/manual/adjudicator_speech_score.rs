//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0-rc.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "adjudicator_speech_score")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub adjudicator_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub ballot_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub speech_role: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub speech_position: i32,
    pub manual_total_score: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ballot::Entity",
        from = "Column::BallotId",
        to = "super::ballot::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Ballot,
    #[sea_orm(
        belongs_to = "super::ballot_adjudicator::Entity",
        from = "Column::AdjudicatorId",
        to = "super::ballot_adjudicator::Column::BallotId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    BallotAdjudicator,
    #[sea_orm(
        belongs_to = "super::ballot_speech::Entity",
        from = "Column::BallotId",
        to = "super::ballot_speech::Column::BallotId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    #[sea_orm(has_many = "super::ballot_speech::Entity")]
    BallotSpeech,
}

impl Related<super::ballot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ballot.def()
    }
}

impl Related<super::ballot_adjudicator::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BallotAdjudicator.def()
    }
}

impl Related<super::ballot_speech::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BallotSpeech.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
