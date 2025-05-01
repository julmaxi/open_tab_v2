use sea_orm_migration::{prelude::*, schema::*, sea_orm::DbBackend};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub enum BallotSpeech {
    Table,
    IsOptOut,
}

#[derive(DeriveIden)]
pub enum Participant {
    Table,
    TournamentId,
}

#[derive(DeriveIden)]
pub enum Tournament {
    Table,
    Uuid,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Not ideal, but sqlite schema changes are too annoying to do
        if manager.get_database_backend() != DbBackend::Sqlite {
            manager
            .alter_table(
                Table::alter()
                    .table(BallotSpeech::Table)
                    .modify_column(
                        ColumnDef::new(BallotSpeech::IsOptOut)
                            .boolean()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(Participant::Table)
                        .add_foreign_key(
                            TableForeignKey::new()
                                .name("fk_participant_tournament")
                                .from_tbl(Participant::Table)
                                .from_col(Participant::TournamentId)
                                .to_tbl(Tournament::Table)
                                .to_col(Tournament::Uuid)
                                .on_delete(ForeignKeyAction::Cascade)
                                .on_update(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;
        }
    
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(BallotSpeech::Table)
                    .modify_column(
                        ColumnDef::new(BallotSpeech::IsOptOut)
                            .boolean()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

            manager
                .alter_table(
                    Table::alter()
                        .table(Participant::Table)
                        .drop_foreign_key(Alias::new("fk_participant_tournament"))
                        .to_owned(),
                )
                .await
    }
}
