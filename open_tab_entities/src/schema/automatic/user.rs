//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    #[sea_orm(unique)]
    pub user_email: Option<String>,
    pub password_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_access_key::Entity")]
    UserAccessKey,
    #[sea_orm(has_many = "super::user_associated_institution::Entity")]
    UserAssociatedInstitution,
    #[sea_orm(has_many = "super::user_participant::Entity")]
    UserParticipant,
    #[sea_orm(has_many = "super::user_tournament::Entity")]
    UserTournament,
}

impl Related<super::user_access_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAccessKey.def()
    }
}

impl Related<super::user_associated_institution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAssociatedInstitution.def()
    }
}

impl Related<super::user_participant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserParticipant.def()
    }
}

impl Related<super::user_tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserTournament.def()
    }
}

impl Related<super::participant::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_participant::Relation::Participant.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_participant::Relation::User.def().rev())
    }
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_tournament::Relation::Tournament.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_tournament::Relation::User.def().rev())
    }
}

impl Related<super::well_known_institution::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_associated_institution::Relation::WellKnownInstitution.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::user_associated_institution::Relation::User
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
