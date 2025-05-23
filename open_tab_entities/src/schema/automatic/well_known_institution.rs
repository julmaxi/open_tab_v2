//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "well_known_institution")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub name: String,
    #[sea_orm(unique)]
    pub short_name: String,
    pub tiny_image: Option<Uuid>,
    pub header_image: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::institution_alias::Entity")]
    InstitutionAlias,
    #[sea_orm(has_many = "super::user_associated_institution::Entity")]
    UserAssociatedInstitution,
}

impl Related<super::institution_alias::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InstitutionAlias.def()
    }
}

impl Related<super::user_associated_institution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAssociatedInstitution.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_associated_institution::Relation::User.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::user_associated_institution::Relation::WellKnownInstitution
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
