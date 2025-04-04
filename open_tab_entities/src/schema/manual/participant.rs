//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0-rc.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "participant")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub name: String,
    pub registration_key: Option<Vec<u8>>,
    pub is_anonymous: bool,
    pub break_category_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::adjudicator::Entity")]
    Adjudicator,
    #[sea_orm(has_one = "super::speaker::Entity")]
    Speaker,
    #[sea_orm(has_many = "super::participant_tournament_institution::Entity")]
    TournamentInstitution,
    #[sea_orm(
        belongs_to = "super::tournament::Entity",
        from = "Column::TournamentId",
        to = "super::tournament::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Tournament,
    #[
        sea_orm(
            belongs_to = "super::tournament_break_category::Entity",
            from = "Column::BreakCategoryId",
            to = "super::tournament_break_category::Column::Uuid",
            on_update = "Cascade",
            on_delete = "SetNull"
        )]
    BreakCategory,
 }

impl Related<super::adjudicator::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Adjudicator.def()
    }
}

impl Related<super::speaker::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Speaker.def()
    }
}

impl Related<super::participant_tournament_institution::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentInstitution.def()
    }
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
