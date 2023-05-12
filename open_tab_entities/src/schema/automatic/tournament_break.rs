//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0-rc.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_break")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub break_type: String,
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
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
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

impl Related<super::tournament_round::Entity> for Entity {
    fn to() -> RelationDef {
        super::tournament_break_child_round::Relation::TournamentRound.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::tournament_break_child_round::Relation::TournamentBreak
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}