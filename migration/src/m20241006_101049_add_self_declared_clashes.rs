use sea_orm::{DbBackend, Statement};
use sea_orm_migration::{prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;


#[derive(DeriveIden)]
enum ClashDeclaration {
    Table,
    Uuid,
    WasSeen,
    SourceParticipantId,
    TargetParticipantId,
    Severity,
    IsRetracted
}

#[derive(DeriveIden)]
enum InstitutionDeclaration {
    Table,
    Uuid,
    WasSeen,
    SourceParticipantId,
    TournamentInstitutionId,
    Severity,
    IsRetracted
}

#[derive(DeriveIden)]
enum Tournament {
    Table,
    AllowSelfDeclaredClashes,
    AllowSpeakerSelfDeclaredClashes,
    ShowDeclaredClashes
}

#[derive(DeriveIden)]
enum TournamentInstitution {
    Table,
    Uuid
}

#[derive(DeriveIden)]
enum Participant {
    Table,
    Uuid
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create ClashDeclaration table
        manager
            .create_table(
                TableCreateStatement::new()
                    .table(ClashDeclaration::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ClashDeclaration::Uuid)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ClashDeclaration::WasSeen).boolean().not_null().default(false))
                    .col(
                        ColumnDef::new(ClashDeclaration::SourceParticipantId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ClashDeclaration::TargetParticipantId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ClashDeclaration::Severity).integer().not_null())
                    .col(ColumnDef::new(ClashDeclaration::IsRetracted).boolean().not_null().default(false))
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(ClashDeclaration::Table)
                            .from_col(ClashDeclaration::SourceParticipantId)
                            .to_tbl(Participant::Table)
                            .to_col(Participant::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(ClashDeclaration::Table)
                            .from_col(ClashDeclaration::TargetParticipantId)
                            .to_tbl(Participant::Table)
                            .to_col(Participant::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        // Create InstitutionDeclaration table
        manager
            .create_table(
                TableCreateStatement::new()
                    .table(InstitutionDeclaration::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InstitutionDeclaration::Uuid)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(InstitutionDeclaration::WasSeen).boolean().not_null())
                    .col(
                        ColumnDef::new(InstitutionDeclaration::SourceParticipantId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InstitutionDeclaration::TournamentInstitutionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(InstitutionDeclaration::Severity).integer().not_null())
                    .col(ColumnDef::new(ClashDeclaration::IsRetracted).boolean().not_null().default(false))
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(InstitutionDeclaration::Table)
                            .from_col(InstitutionDeclaration::SourceParticipantId)
                            .to_tbl(Participant::Table)
                            .to_col(Participant::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from_tbl(InstitutionDeclaration::Table)
                            .from_col(InstitutionDeclaration::TournamentInstitutionId)
                            .to_tbl(TournamentInstitution::Table)
                            .to_col(TournamentInstitution::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        manager.create_index(
            IndexCreateStatement::new()
                .name("idx-clash-declaration-participant-id")
                .table(ClashDeclaration::Table)
                .col(InstitutionDeclaration::SourceParticipantId)
                .to_owned(),
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
                .name("idx-institution-declaration-participant-id")
                .table(InstitutionDeclaration::Table)
                .col(InstitutionDeclaration::SourceParticipantId)
                .to_owned(),
        ).await?;

        let new_tournament_columns = vec![
            ColumnDef::new(Tournament::AllowSelfDeclaredClashes)
                .boolean()
                .not_null()
                .default(false)
                .to_owned(),
            ColumnDef::new(Tournament::AllowSpeakerSelfDeclaredClashes)
                .boolean()
                .not_null()
                .default(false)
                .to_owned(),
            ColumnDef::new(Tournament::ShowDeclaredClashes)
                .boolean()
                .not_null()
                .default(false)
                .to_owned(),
        ];


        match manager.get_database_backend() {
            DbBackend::Sqlite => {
                for mut column in new_tournament_columns {
                    manager
                        .alter_table(
                            TableAlterStatement::new()
                                .table(Tournament::Table)
                                .add_column(&mut column)
                                .to_owned(),
                        )
                        .await?;
                }
            }
            _ => {
                let mut statement = TableAlterStatement::new();
                statement.table(Tournament::Table);
                for mut column in new_tournament_columns {
                    statement.add_column(&mut column);
                }
                manager.alter_table(statement.to_owned()).await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order of creation
        manager
            .drop_table(TableDropStatement::new().table(InstitutionDeclaration::Table).to_owned())
            .await?;
        manager
            .drop_table(TableDropStatement::new().table(ClashDeclaration::Table).to_owned())
            .await?;

        Ok(())
    }
}
