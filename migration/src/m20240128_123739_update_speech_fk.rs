use sea_orm_migration::{manager, prelude::*, sea_orm::Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
pub enum BallotSpeech {
    Table,
    BallotId,
    SpeakerId,
    Position,
    Role,
    IsOptOut
}

#[derive(Iden)]
pub enum BallotSpeechTemp {
    Table,
    SpeakerId,
}

#[derive(Iden)]
pub enum Speaker {
    Table,
    Uuid,
}

#[derive(Iden)]
enum Ballot {
    Table,
    Uuid,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager.get_database_backend() {
            sea_orm::DatabaseBackend::Sqlite => {
                manager.get_connection().execute(Statement::from_string(manager.get_database_backend(), "PRAGMA foreign_keys=off;")).await?;

                manager.rename_table(
                    Table::rename()
                        .table(BallotSpeech::Table, BallotSpeechTemp::Table)
                        .to_owned(),
                ).await?;

                manager
                .create_table(
                    sea_query::Table::create()
                        .table(BallotSpeech::Table)
                        .if_not_exists()
                        .col(ColumnDef::new(BallotSpeech::BallotId).uuid().not_null())
                        .col(ColumnDef::new(BallotSpeech::Position).integer().not_null())
                        .col(ColumnDef::new(BallotSpeech::Role).string_len(1).not_null())
                        .col(ColumnDef::new(BallotSpeech::SpeakerId).uuid())
                        .col(ColumnDef::new(BallotSpeech::IsOptOut).boolean().default(false))
                        .primary_key(
                            Index::create()
                                .name("pk-speech")
                                .col(BallotSpeech::BallotId)
                                .col(BallotSpeech::Role)
                                .col(BallotSpeech::Position)
                                .primary(),
                        )
                        .foreign_key(
                            ForeignKeyCreateStatement::new()
                                .name("fk-speech-ballot")
                                .from_tbl(BallotSpeech::Table)
                                .from_col(BallotSpeech::BallotId)
                                .to_tbl(Ballot::Table)
                                .to_col(Ballot::Uuid)
                                .on_delete(ForeignKeyAction::Cascade)
                                .on_update(ForeignKeyAction::Cascade),
                        )
                        .foreign_key(
                            ForeignKeyCreateStatement::new()
                                .name("fk-speech-speaker")
                                .from_tbl(BallotSpeech::Table)
                                .from_col(BallotSpeech::SpeakerId)
                                .to_tbl(Speaker::Table)
                                .to_col(Speaker::Uuid)
                                .on_delete(ForeignKeyAction::SetNull)
                                .on_update(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

                manager.get_connection().execute(Statement::from_string(manager.get_database_backend(), "INSERT INTO ballot_speech (ballot_id, position, role, speaker_id, is_opt_out) SELECT ballot_id, position, role, speaker_id, is_opt_out FROM ballot_speech_temp;")).await?;
    
            }
            _ => {
                manager
                    .drop_foreign_key(
                        ForeignKey::drop()
                            .name("fk-speech-speaker")
                            .table(BallotSpeech::Table)
                            .to_owned(),
                    )
                    .await?;

                manager
                    .create_foreign_key(
                        ForeignKey::create()
                            .name("fk-speech-speaker")
                            .from(BallotSpeech::Table, BallotSpeech::SpeakerId)
                            .to(Speaker::Table, Speaker::Uuid)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade)
                            .to_owned(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        manager
            .drop_table(Table::drop().table(Post::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Post {
    Table,
    Id,
    Title,
    Text,
}
