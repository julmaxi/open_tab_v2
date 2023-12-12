use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum DebateBackupBallot {
    Table,
    DebateId,
    BallotId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts

        manager.create_index(
            Index::create()
                .table(DebateBackupBallot::Table)
                .name("debate_backup_ballot_debate_id")
                .col(DebateBackupBallot::DebateId)
                .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
                .table(DebateBackupBallot::Table)
                .name("debate_backup_ballot_ballot_id")
                .col(DebateBackupBallot::BallotId)
                .unique()
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_index(
            Index::drop()
                .table(DebateBackupBallot::Table)
                .name("debate_backup_ballot_debate_id")
                .to_owned(),
        ).await?;
        manager.drop_index(
            Index::drop()
                .table(DebateBackupBallot::Table)
                .name("debate_backup_ballot_ballot_id")
                .to_owned(),
        ).await?;

        Ok(())
    }
}
