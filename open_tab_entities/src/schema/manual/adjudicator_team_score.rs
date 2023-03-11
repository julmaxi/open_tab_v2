//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0-rc.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "adjudicator_team_score")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub adjudicator_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub ballot_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: String,
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
        to = "super::ballot_adjudicator::Column::AdjudicatorId",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    BallotAdjudicator,
    #[sea_orm(
        belongs_to = "super::ballot_team::Entity",
        from = "Column::Ballot",
        from = "Column::RoleId",
        to = "super::ballot_team::Column::Ballot",
        to = "super::ballot_team::Column::Role",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    BallotTeam
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

impl Related<super::ballot_team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BallotTeam.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
