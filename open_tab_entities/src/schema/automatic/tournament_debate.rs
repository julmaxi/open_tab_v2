//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0-rc.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_debate")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub round_id: Uuid,
    pub index: i32,
    pub ballot_id: Uuid,
    pub venue_id: Option<Uuid>,
    pub is_motion_released_to_non_aligned: bool,
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
    #[sea_orm(has_many = "super::debate_backup_ballot::Entity")]
    DebateBackupBallot,
    #[sea_orm(
        belongs_to = "super::tournament_round::Entity",
        from = "Column::RoundId",
        to = "super::tournament_round::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentRound,
}

impl Related<super::ballot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ballot.def()
    }
}

impl Related<super::debate_backup_ballot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DebateBackupBallot.def()
    }
}

impl Related<super::tournament_round::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentRound.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
