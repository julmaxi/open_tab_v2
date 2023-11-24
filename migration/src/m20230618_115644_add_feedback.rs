use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum Tournament {
    Table,
    Uuid,
}

#[derive(Iden)]
enum Participant {
    Table,
    Uuid,
}

#[derive(Iden)]
pub enum Team {
    Table,
    Uuid
}

#[derive(Iden)]
pub enum TournamentDebate {
    Table,
    Uuid
}

#[derive(Iden)]
enum FeedbackForm {
    Table,
    Uuid,
    TournamentId,
    Name,
    ShowChairsForWings,
    ShowChairsForPresidents,
    ShowWingsForChairs,
    ShowWingsForPresidents,
    ShowWingsForWings,
    ShowPresidentsForChairs,
    ShowPresidentsForWings,
    ShowTeamsForChairs,
    ShowTeamsForPresidents,
    ShowTeamsForWings,
    ShowNonAlignedForChairs,
    ShowNonAlignedForPresidents,
    ShowNonAlignedForWings,
}



#[derive(Iden)]
enum FeedbackQuestion {
    Table,
    Uuid,
    TournamentId,
    ShortName,
    FullName,
    Description,
    QuestionConfig
}

#[derive(Iden)]
enum FeedbackFormQuestion {
    Table,
    FeedbackFormId,
    FeedbackQuestionId,
    Index
}



#[derive(Iden)]
enum FeedbackResponse {
    Table,
    Uuid,
    AuthorParticipantId,
    TargetParticipantId,
    SourceTeamId,
    SourceParticipantId,
    SourceDebateId,
}

#[derive(Iden)]
enum FeedbackResponseValue {
    Table,
    ResponseId,
    QuestionId,
    IntValue,
    BoolValue,
    StringValue,
}

#[derive(DeriveMigrationName)]
pub struct Migration;


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FeedbackForm::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FeedbackForm::Uuid)
                            .uuid()
                            .not_null()
                            .primary_key()
                    )
                    .col(ColumnDef::new(FeedbackForm::TournamentId).uuid())
                    .col(ColumnDef::new(FeedbackForm::Name).string().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowChairsForPresidents).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowChairsForWings).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowWingsForChairs).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowWingsForPresidents).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowWingsForWings).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowPresidentsForChairs).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowPresidentsForWings).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowTeamsForChairs).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowTeamsForPresidents).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowTeamsForWings).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowNonAlignedForChairs).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowNonAlignedForPresidents).boolean().not_null())
                    .col(ColumnDef::new(FeedbackForm::ShowNonAlignedForWings).boolean().not_null())

                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_feedback_form-tournament")
                            .from(FeedbackForm::Table, FeedbackForm::TournamentId)
                            .to(Tournament::Table, Tournament::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )    
                    .to_owned()
            )
            .await?;

        manager.create_table(
            Table::create()
                .table(FeedbackQuestion::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(FeedbackQuestion::Uuid)
                        .uuid()
                        .not_null()
                        .primary_key()
                )
                .col(ColumnDef::new(FeedbackQuestion::TournamentId).uuid())
                .col(ColumnDef::new(FeedbackQuestion::ShortName).string().not_null())
                .col(ColumnDef::new(FeedbackQuestion::FullName).string().not_null())
                .col(ColumnDef::new(FeedbackQuestion::Description).string().not_null())
                .col(ColumnDef::new(FeedbackQuestion::QuestionConfig).string().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_question-tournament")
                        .from(FeedbackQuestion::Table, FeedbackQuestion::TournamentId)
                        .to(Tournament::Table, Tournament::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(FeedbackFormQuestion::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(FeedbackFormQuestion::FeedbackFormId)
                        .uuid()
                        .not_null()
                )
                .col(
                    ColumnDef::new(FeedbackFormQuestion::FeedbackQuestionId)
                        .uuid()
                        .not_null()
                )
                .col(ColumnDef::new(FeedbackFormQuestion::Index).integer().not_null())
                .primary_key(
                    Index::create()
                        .name("pk-feedback_form_question")
                        .col(FeedbackFormQuestion::FeedbackFormId)
                        .col(FeedbackFormQuestion::FeedbackQuestionId)
                        .primary(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_form_question-feedback_form")
                        .from(FeedbackFormQuestion::Table, FeedbackFormQuestion::FeedbackFormId)
                        .to(FeedbackForm::Table, FeedbackForm::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_form_question-feedback_question")
                        .from(FeedbackFormQuestion::Table, FeedbackFormQuestion::FeedbackQuestionId)
                        .to(FeedbackQuestion::Table, FeedbackQuestion::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_table(
            Table::create()
                .table(FeedbackResponse::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(FeedbackResponse::Uuid)
                        .uuid()
                        .not_null()
                        .primary_key()
                )
                .col(ColumnDef::new(FeedbackResponse::AuthorParticipantId).uuid().not_null())
                .col(ColumnDef::new(FeedbackResponse::TargetParticipantId).uuid().not_null())
                .col(ColumnDef::new(FeedbackResponse::SourceTeamId).uuid())
                .col(ColumnDef::new(FeedbackResponse::SourceParticipantId).uuid())
                .col(ColumnDef::new(FeedbackResponse::SourceDebateId).uuid().not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_response-author_participant")
                        .from(FeedbackResponse::Table, FeedbackResponse::AuthorParticipantId)
                        .to(Participant::Table, Participant::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_response-target_participant")
                        .from(FeedbackResponse::Table, FeedbackResponse::TargetParticipantId)
                        .to(Participant::Table, Participant::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_response-source_team")
                        .from(FeedbackResponse::Table, FeedbackResponse::SourceTeamId)
                        .to(Team::Table, Team::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_response-source_participant")
                        .from(FeedbackResponse::Table, FeedbackResponse::SourceParticipantId)
                        .to(Participant::Table, Participant::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_response-debate_round")
                        .from(FeedbackResponse::Table, FeedbackResponse::SourceDebateId)
                        .to(TournamentDebate::Table, TournamentDebate::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
            ).await?;

        manager.create_table(
            Table::create()
                .table(FeedbackResponseValue::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(FeedbackResponseValue::ResponseId)
                        .uuid()
                        .not_null()
                )
                .col(
                    ColumnDef::new(FeedbackResponseValue::QuestionId)
                        .uuid()
                        .not_null()
                )
                .col(ColumnDef::new(FeedbackResponseValue::IntValue).integer())
                .col(ColumnDef::new(FeedbackResponseValue::StringValue).string())
                .col(ColumnDef::new(FeedbackResponseValue::BoolValue).boolean())
                .primary_key(
                    Index::create()
                        .name("pk_feedback_response_value")
                        .col(FeedbackResponseValue::ResponseId)
                        .col(FeedbackResponseValue::QuestionId)
                        .primary(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_response_value-response")
                        .from(FeedbackResponseValue::Table, FeedbackResponseValue::ResponseId)
                        .to(FeedbackResponse::Table, FeedbackResponse::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_feedback_response_value-question")
                        .from(FeedbackResponseValue::Table, FeedbackResponseValue::QuestionId)
                        .to(FeedbackQuestion::Table, FeedbackQuestion::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
            ).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        /*manager
            .drop_table(Table::drop().table(Post::Table).to_owned())
            .await*/
    }
}
