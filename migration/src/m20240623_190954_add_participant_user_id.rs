use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Participant {
    Table,
    UserId
}

#[derive(Iden)]
enum User {
    Table,
    Uuid
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(Participant::Table)
                    //This can not be a foreign key.
                    //User as a concept is outside of the scope of the tournament.
                    //If it was a foreign key, a sync would fail if, for example, a
                    //user was deleted.
                    .add_column(ColumnDef::new(Participant::UserId).uuid())
                    .to_owned()
            )
            .await?;

        manager.
            create_index(
                Index::create()
                    .name("participant_user_id")
                    .table(Participant::Table)
                    .col(Participant::UserId)
                    .to_owned()
            ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                TableAlterStatement::new()
                    .table(Participant::Table)
                    .drop_column(Participant::UserId)
                    .to_owned()
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("participant_user_id")
                    .table(Participant::Table)
                    .to_owned()
            ).await
    }
}
