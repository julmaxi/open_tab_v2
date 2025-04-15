//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_plan_node_round")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub plan_node_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false, unique)]
    pub round_id: Uuid,
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tournament_plan_node::Entity",
        from = "Column::PlanNodeId",
        to = "super::tournament_plan_node::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentPlanNode,
    #[sea_orm(
        belongs_to = "super::tournament_round::Entity",
        from = "Column::RoundId",
        to = "super::tournament_round::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentRound,
}

impl Related<super::tournament_plan_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentPlanNode.def()
    }
}

impl Related<super::tournament_round::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentRound.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
