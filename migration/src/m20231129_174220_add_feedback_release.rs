use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum TournamentRound {
    Table,
    FeedbackReleaseTime
}


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
            .table(TournamentRound::Table)
            .add_column(ColumnDef::new(TournamentRound::FeedbackReleaseTime).date_time())
            .to_owned()
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(
            Table::alter()
            .table(TournamentRound::Table)
            .drop_column(TournamentRound::FeedbackReleaseTime)
            .to_owned()
        ).await
    }
}
