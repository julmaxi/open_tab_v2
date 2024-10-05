use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Tournament {
    Table,
    LastModified
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let now = chrono::Utc::now();
        manager
        .alter_table(
            TableAlterStatement::new()
                .table(Tournament::Table)
                .add_column(ColumnDef::new(Tournament::LastModified)
                .date_time()
                .not_null()
                //This default is only used to migrate existing data.
                .default(now.to_rfc3339()))
                .to_owned(),
        )
        .await?;

        Ok(())    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();
    }
}
