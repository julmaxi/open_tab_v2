use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Tournament {
    Table,
    Uuid
}

#[derive(Iden)]
enum User {
    Table,
    Uuid,
    PasswordHash
}

#[derive(Iden)]
enum UserTournament {
    Table,
    UserId,
    TournamentId
}

#[derive(Iden)]
enum UserAccessKey {
    Table,
    KeyHash,
    UserId,
    TournamentId
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(User::Uuid)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(User::PasswordHash)
                            .string_len(60)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserAccessKey::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserAccessKey::KeyHash)
                        .string_len(60)
                        .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserAccessKey::UserId)
                            .uuid()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(UserAccessKey::TournamentId)
                            .uuid()
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-user-key_user")
                            .from_tbl(UserAccessKey::Table)
                            .from_col(UserAccessKey::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-user-key_tournament")
                            .from_tbl(UserAccessKey::Table)
                            .from_col(UserAccessKey::TournamentId)
                            .to_tbl(Tournament::Table)
                            .to_col(Tournament::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserTournament::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserTournament::UserId)
                            .uuid()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(UserTournament::TournamentId)
                            .uuid()
                            .not_null()
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-user-tournament_user")
                            .from_tbl(UserTournament::Table)
                            .from_col(UserTournament::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-tournament_user-tournament")
                            .from_tbl(UserTournament::Table)
                            .from_col(UserTournament::TournamentId)
                            .to_tbl(Tournament::Table)
                            .to_col(Tournament::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

        manager
            .drop_table(Table::drop().table(Post::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Post {
    Table,
    Id,
    Title,
    Text,
}
