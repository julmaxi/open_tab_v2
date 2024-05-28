use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;


#[derive(Iden)]
enum BallotSpeechTiming {
    Table,
    PauseMilliseconds,
    ResponsePauseMilliseconds,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
        .alter_table(
            TableAlterStatement::new()
                .table(BallotSpeechTiming::Table)
                .add_column(ColumnDef::new(BallotSpeechTiming::PauseMilliseconds).integer().not_null().default(0))
                .to_owned()
        )
        .await?;

        manager
        .alter_table(
            TableAlterStatement::new()
                .table(BallotSpeechTiming::Table)
                .add_column(ColumnDef::new(BallotSpeechTiming::ResponsePauseMilliseconds).integer().not_null().default(0))
                .to_owned()
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        todo!()
    }
}
