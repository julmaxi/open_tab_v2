use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum UserAccessKey {
    Table,
    ExpiryDate,
    IsAccessOnly
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(UserAccessKey::Table)
                    .add_column(ColumnDef::new(UserAccessKey::ExpiryDate).date_time())
                    .to_owned()
            )
            .await?;
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(UserAccessKey::Table)
                    .add_column(ColumnDef::new(UserAccessKey::IsAccessOnly).boolean().not_null().default(false))
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(UserAccessKey::Table)
                    .drop_column(UserAccessKey::ExpiryDate)
                    .drop_column(UserAccessKey::IsAccessOnly)
                    .to_owned()
            )
            .await
    }
}
