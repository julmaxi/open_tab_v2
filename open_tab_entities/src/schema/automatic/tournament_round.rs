//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.6

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tournament_round")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub index: i32,
    pub motion: Option<String>,
    pub info_slide: Option<String>,
    pub draw_release_time: Option<DateTime>,
    pub team_motion_release_time: Option<DateTime>,
    pub debate_start_time: Option<DateTime>,
    pub full_motion_release_time: Option<DateTime>,
    pub round_close_time: Option<DateTime>,
    pub silent_round_results_release_time: Option<DateTime>,
    pub is_silent: bool,
    pub feedback_release_time: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::adjudicator_availability_override::Entity")]
    AdjudicatorAvailabilityOverride,
    #[sea_orm(
        belongs_to = "super::tournament::Entity",
        from = "Column::TournamentId",
        to = "super::tournament::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Tournament,
    #[sea_orm(has_many = "super::tournament_debate::Entity")]
    TournamentDebate,
    #[sea_orm(has_many = "super::tournament_plan_node_round::Entity")]
    TournamentPlanNodeRound,
}

impl Related<super::adjudicator_availability_override::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AdjudicatorAvailabilityOverride.def()
    }
}

impl Related<super::tournament::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tournament.def()
    }
}

impl Related<super::tournament_debate::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentDebate.def()
    }
}

impl Related<super::tournament_plan_node_round::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentPlanNodeRound.def()
    }
}

impl Related<super::adjudicator::Entity> for Entity {
    fn to() -> RelationDef {
        super::adjudicator_availability_override::Relation::Adjudicator.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::adjudicator_availability_override::Relation::TournamentRound
                .def()
                .rev(),
        )
    }
}

impl Related<super::tournament_plan_node::Entity> for Entity {
    fn to() -> RelationDef {
        super::tournament_plan_node_round::Relation::TournamentPlanNode.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::tournament_plan_node_round::Relation::TournamentRound
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
