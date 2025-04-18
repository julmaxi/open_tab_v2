//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_break_team")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub tournament_break_id: Uuid,
    pub team_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::TeamId",
        to = "super::team::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Team,
    #[sea_orm(
        belongs_to = "super::tournament_break::Entity",
        from = "Column::TournamentBreakId",
        to = "super::tournament_break::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentBreak,
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

impl ActiveModelBehavior for ActiveModel {}
