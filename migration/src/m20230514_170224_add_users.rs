use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Tournament {
    Table,
    Uuid
}

#[derive(Iden)]
enum Participant {
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
enum UserParticipant {
    Table,
    UserId,
    ParticipantId
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
                    .primary_key(Index::create().col(UserTournament::UserId).col(UserTournament::TournamentId).primary())
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

            manager
            .create_table(
                Table::create()
                    .table(UserParticipant::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserParticipant::UserId)
                            .uuid()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(UserParticipant::ParticipantId)
                            .uuid()
                            .not_null()
                    )
                    .primary_key(Index::create().col(UserParticipant::UserId).col(UserParticipant::ParticipantId).primary())
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-user_participant-user")
                            .from_tbl(UserTournament::Table)
                            .from_col(UserTournament::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .name("fk-user_participant-participant")
                            .from_tbl(UserParticipant::Table)
                            .from_col(UserParticipant::ParticipantId)
                            .to_tbl(Participant::Table)
                            .to_col(Participant::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            )
            .await?;

            manager.create_index(
                IndexCreateStatement::new()
                .name("idx-user_participant-participant_id")
                .table(UserParticipant::Table)
                .col(UserParticipant::ParticipantId)
                .to_owned()
            ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        todo!();
    }
}
