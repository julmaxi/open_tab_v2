use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum BallotSpeechTiming {
    Table,
    Uuid,
    SpeechBallotId,
    SpeechRole,
    SpeechPosition,
    StartTime,
    EndTime,
    ResponseStartTime,
    ResponseEndTime,
}

#[derive(Iden)]
enum BallotSpeech {
    Table,
    BallotId,
    Role,
    Position
}


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(BallotSpeechTiming::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(BallotSpeechTiming::Uuid)
                        .uuid()
                        .not_null()
                        .primary_key()
                )
                .col(
                    ColumnDef::new(BallotSpeechTiming::SpeechBallotId)
                        .uuid()
                        .not_null()
                )
                .col(
                    ColumnDef::new(BallotSpeechTiming::SpeechRole)
                        .string_len(1)
                        .not_null()
                )
                .col(
                    ColumnDef::new(BallotSpeechTiming::SpeechPosition)
                        .integer()
                        .not_null()
                )
                .col(
                    ColumnDef::new(BallotSpeechTiming::StartTime)
                        .date_time()
                )
                .col(
                    ColumnDef::new(BallotSpeechTiming::EndTime)
                        .date_time()
                )
                .col(
                    ColumnDef::new(BallotSpeechTiming::ResponseStartTime)
                        .date_time()
                )
                .col(
                    ColumnDef::new(BallotSpeechTiming::ResponseEndTime)
                        .date_time()
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_ballot_speech_timing_ballot_speech")
                        .from(BallotSpeechTiming::Table, (
                            BallotSpeechTiming::SpeechBallotId,
                            BallotSpeechTiming::SpeechRole,
                            BallotSpeechTiming::SpeechPosition
                        ))
                        .to(BallotSpeech::Table, (
                            BallotSpeech::BallotId,
                            BallotSpeech::Role,
                            BallotSpeech::Position
                        ))
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                ).to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .table(BallotSpeechTiming::Table)
                .name("idx_ballot_speech_timing-ballot_speech")
                .col(BallotSpeechTiming::SpeechBallotId)
                .col(BallotSpeechTiming::SpeechRole)
                .col(BallotSpeechTiming::SpeechPosition)
                .unique()
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .table(BallotSpeechTiming::Table)
                .name("idx_ballot_speech_timing-ballot")
                .col(BallotSpeechTiming::SpeechBallotId)
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
        .drop_table(
            TableDropStatement::new()
                .table(BallotSpeechTiming::Table)
                .to_owned()
        )
        .await
    }
}
