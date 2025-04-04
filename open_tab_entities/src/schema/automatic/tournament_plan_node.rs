//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.6

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_plan_node")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub config: String,
    pub tournament_id: Uuid,
    pub break_id: Option<Uuid>,
    pub suggested_award_title: Option<String>,
    pub max_breaking_adjudicator_count: Option<i32>,
    pub is_only_award: bool,
    pub suggested_award_prestige: Option<i32>,
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
    #[sea_orm(
        belongs_to = "super::tournament_break::Entity",
        from = "Column::BreakId",
        to = "super::tournament_break::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentBreak,
    #[sea_orm(has_many = "super::tournament_break_eligible_category::Entity")]
    TournamentBreakEligibleCategory,
    #[sea_orm(has_many = "super::tournament_plan_node_round::Entity")]
    TournamentPlanNodeRound,
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
    }
}

impl Related<super::tournament_break::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentBreak.def()
    }
}

impl Related<super::tournament_break_eligible_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentBreakEligibleCategory.def()
    }
}

impl Related<super::tournament_plan_node_round::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentPlanNodeRound.def()
    }
}

impl Related<super::tournament_break_category::Entity> for Entity {
    fn to() -> RelationDef {
        super::tournament_break_eligible_category::Relation::TournamentBreakCategory.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::tournament_break_eligible_category::Relation::TournamentPlanNode
                .def()
                .rev(),
        )
    }
}

impl Related<super::tournament_round::Entity> for Entity {
    fn to() -> RelationDef {
        super::tournament_plan_node_round::Relation::TournamentRound.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::tournament_plan_node_round::Relation::TournamentPlanNode
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
