use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TournamentBreak::Table)
                    .add_column(
                        ColumnDef::new(TournamentBreak::ReleaseTime)
                            .timestamp()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TournamentBreak::Table)
                    .drop_column(TournamentBreak::ReleaseTime)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TournamentBreak {
    Table,
    ReleaseTime
}