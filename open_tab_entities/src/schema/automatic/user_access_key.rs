//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.6

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_access_key")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub key_hash: String,
    pub user_id: Uuid,
    pub tournament_id: Option<Uuid>,
    pub expiry_date: Option<DateTime>,
    pub is_access_only: bool
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
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
