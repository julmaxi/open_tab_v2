use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum FeedbackQuestion {
    Table,
    IsConfidential,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            TableAlterStatement::new()
                .table(FeedbackQuestion::Table)
                .add_column(ColumnDef::new(FeedbackQuestion::IsConfidential).boolean().default(false).not_null())
                .to_owned(),
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(FeedbackQuestion::Table)
                    .drop_column(FeedbackQuestion::IsConfidential)
                    .to_owned(),
            ).await
    }
}

#[derive(DeriveIden)]
enum Post {
    Table,
    Id,
    Title,
    Text,
}
