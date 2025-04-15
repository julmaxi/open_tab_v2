//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "participant_tournament_institution")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub participant_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub institution_id: Uuid,
    pub clash_severity: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::participant::Entity",
        from = "Column::ParticipantId",
        to = "super::participant::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Participant,
    #[sea_orm(
        belongs_to = "super::tournament_institution::Entity",
        from = "Column::InstitutionId",
        to = "super::tournament_institution::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentInstitution,
}

impl Related<super::participant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Participant.def()
    }
}

impl Related<super::tournament_institution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentInstitution.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
