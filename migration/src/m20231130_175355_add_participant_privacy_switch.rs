use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Participant {
    Table,
    IsAnonymous
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(Participant::Table)
                    .add_column(ColumnDef::new(Participant::IsAnonymous).not_null().boolean().default(false))
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(Participant::Table)
                    .drop_column(Participant::IsAnonymous)
                    .to_owned()
            )
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
