//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0-rc.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub password_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_access_key::Entity")]
    UserAccessKey,
}

impl Related<super::user_access_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAccessKey.def()
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

impl Related<super::participant::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_participant::Relation::Participant.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::user_participant::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}