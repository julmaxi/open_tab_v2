//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.6

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_break_eligible_category")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub tournament_break_category_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub tournament_plan_node_id: Uuid,
    pub config: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tournament_break_category::Entity",
        from = "Column::TournamentBreakCategoryId",
        to = "super::tournament_break_category::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentBreakCategory,
    #[sea_orm(
        belongs_to = "super::tournament_plan_node::Entity",
        from = "Column::TournamentPlanNodeId",
        to = "super::tournament_plan_node::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentPlanNode,
}

impl Related<super::tournament_break_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentBreakCategory.def()
    }
}

impl Related<super::tournament_plan_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentPlanNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
