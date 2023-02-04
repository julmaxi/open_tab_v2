use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220710_203703_add_tournament"
    }
}


#[derive(Iden)]
enum Tournament {
    Table,
    Uuid
}


#[derive(Iden)]
enum TournamentRound {
    Table,
    Uuid,
    TournamentId,
    Index
}



#[derive(Iden)]
enum TournamentLog {
    Table,
    Uuid,
    TournamentId,
    SequenceIdx,
    Timestamp,
    TargetType,
    TargetUuid,
}

#[derive(Iden)]
enum TournamentRemote {
    Table,
    Uuid,
    TournamentId,
    Url,
    LastKnownChange,
}



#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            sea_query::Table::create()
                .table(Tournament::Table)
                .if_not_exists()
                .col(ColumnDef::new(Tournament::Uuid).uuid().not_null().primary_key())
                .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(TournamentLog::Table)
                .if_not_exists()
                .col(ColumnDef::new(TournamentLog::Uuid).uuid().primary_key())
                .col(ColumnDef::new(TournamentLog::TournamentId).uuid())
                .col(ColumnDef::new(TournamentLog::SequenceIdx).integer().not_null())
                .col(ColumnDef::new(TournamentLog::Timestamp).timestamp().not_null())
                .col(ColumnDef::new(TournamentLog::TargetType).string().not_null())
                .col(ColumnDef::new(TournamentLog::TargetUuid).uuid().not_null())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-log-tournament")
                        .from_tbl(TournamentLog::Table)
                        .from_col(TournamentLog::TournamentId)
                        .to_tbl(Tournament::Table)
                        .to_col(Tournament::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;
        
        manager.create_table(
            sea_query::Table::create()
                .table(TournamentRemote::Table)
                .if_not_exists()
                .col(ColumnDef::new(TournamentRemote::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(TournamentRemote::TournamentId).uuid().not_null())
                .col(ColumnDef::new(TournamentRemote::Url).string().not_null())
                .col(ColumnDef::new(TournamentRemote::LastKnownChange).uuid())
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-log_tournament-idx")
            .table(TournamentLog::Table)
            .col(TournamentLog::TournamentId)
            .to_owned()
        ).await?;

        return Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        todo!()
    }
}
