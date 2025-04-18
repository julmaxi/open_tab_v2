//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_break")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub break_award_title: Option<String>,
    pub break_award_prestige: Option<i32>,
    pub award_series_key: Option<String>,
    pub release_time: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tournament::Entity",
        from = "Column::TournamentId",
        to = "super::tournament::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Tournament,
    #[sea_orm(has_many = "super::tournament_break_adjudicator::Entity")]
    TournamentBreakAdjudicator,
    #[sea_orm(has_many = "super::tournament_break_speaker::Entity")]
    TournamentBreakSpeaker,
    #[sea_orm(has_many = "super::tournament_break_team::Entity")]
    TournamentBreakTeam,
    #[sea_orm(has_many = "super::tournament_plan_node::Entity")]
    TournamentPlanNode,
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
    }
}

impl Related<super::tournament_break_adjudicator::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentBreakAdjudicator.def()
    }
}

impl Related<super::tournament_break_speaker::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentBreakSpeaker.def()
    }
}

impl Related<super::tournament_break_team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentBreakTeam.def()
    }
}

impl Related<super::tournament_plan_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentPlanNode.def()
    }
}

impl Related<super::adjudicator::Entity> for Entity {
    fn to() -> RelationDef {
        super::tournament_break_adjudicator::Relation::Adjudicator.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::tournament_break_adjudicator::Relation::TournamentBreak
                .def()
                .rev(),
        )
    }
}

impl Related<super::speaker::Entity> for Entity {
    fn to() -> RelationDef {
        super::tournament_break_speaker::Relation::Speaker.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::tournament_break_speaker::Relation::TournamentBreak
                .def()
                .rev(),
        )
    }
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        super::tournament_break_team::Relation::Team.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::tournament_break_team::Relation::TournamentBreak
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
