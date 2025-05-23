//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.10

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "feedback_response")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub author_participant_id: Uuid,
    pub target_participant_id: Uuid,
    pub source_team_id: Option<Uuid>,
    pub source_participant_id: Option<Uuid>,
    pub source_debate_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::feedback_response_value::Entity")]
    FeedbackResponseValue,
    #[sea_orm(
        belongs_to = "super::participant::Entity",
        from = "Column::AuthorParticipantId",
        to = "super::participant::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Participant3,
    #[sea_orm(
        belongs_to = "super::participant::Entity",
        from = "Column::SourceParticipantId",
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
    #[sea_orm(
        belongs_to = "super::team::Entity",
        from = "Column::SourceTeamId",
        to = "super::team::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Team,
    #[sea_orm(
        belongs_to = "super::tournament_debate::Entity",
        from = "Column::SourceDebateId",
        to = "super::tournament_debate::Column::Uuid",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    TournamentDebate,
}

impl Related<super::feedback_response_value::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FeedbackResponseValue.def()
    }
}

impl Related<super::team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl Related<super::tournament_debate::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TournamentDebate.def()
    }
}

impl Related<super::feedback_question::Entity> for Entity {
    fn to() -> RelationDef {
        super::feedback_response_value::Relation::FeedbackQuestion.def()
    }
    fn via() -> Option<RelationDef> {
        Some(
            super::feedback_response_value::Relation::FeedbackResponse
                .def()
                .rev(),
        )
    }
}

impl ActiveModelBehavior for ActiveModel {}
