//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "participant_clash")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub declaring_participant_id: Uuid,
    pub target_participant_id: Uuid,
    pub clash_severity: i16,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::participant::Entity",
        from = "Column::DeclaringParticipantId",
        to = "super::participant::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Participant2,
    #[sea_orm(
        belongs_to = "super::participant::Entity",
        from = "Column::TargetParticipantId",
        to = "super::participant::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Participant1,
}

impl ActiveModelBehavior for ActiveModel {}
