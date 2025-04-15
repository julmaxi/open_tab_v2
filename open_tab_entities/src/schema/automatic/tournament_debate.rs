//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

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
    pub is_complete: bool,
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
    #[sea_orm(has_many = "super::feedback_response::Entity")]
    FeedbackResponse,
    #[sea_orm(
        belongs_to = "super::tournament_round::Entity",
        from = "Column::RoundId",
        to = "super::tournament_round::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentRound,
    #[sea_orm(
        belongs_to = "super::tournament_venue::Entity",
        from = "Column::VenueId",
        to = "super::tournament_venue::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentVenue,
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

impl Related<super::feedback_response::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FeedbackResponse.def()
    }
}

impl Related<super::tournament_round::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentRound.def()
    }
}

impl Related<super::tournament_venue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentVenue.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
