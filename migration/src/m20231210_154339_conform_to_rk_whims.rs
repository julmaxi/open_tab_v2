use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;


#[derive(DeriveIden)]
pub enum BallotSpeech {
    Table,
    IsOptOut
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(BallotSpeech::Table).add_column(ColumnDef::new(BallotSpeech::IsOptOut).boolean().default(false)).to_owned()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(BallotSpeech::Table).drop_column(BallotSpeech::IsOptOut).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Post {
    Table,
    Id,
    Title,
    Text,
}
