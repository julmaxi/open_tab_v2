use sea_orm::{DbBackend, Statement};
use sea_orm_migration::{prelude::*};

#[derive(DeriveMigrationName)]
pub struct Migration;


#[derive(DeriveIden)]
enum ParticipantClash {
    Table,
    IsUserDeclared,
    IsApproved,
    WasSeen,
}

#[derive(DeriveIden)]
enum Tournament {
    Table,
    AllowSelfDeclaredClashes,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let column_defs = vec![
            ColumnDef::new(ParticipantClash::IsUserDeclared)
                .boolean()
                .not_null()
                .default(false).to_owned(),
            ColumnDef::new(ParticipantClash::IsApproved)
                .boolean()
                .not_null()
                .default(true).to_owned(),
            ColumnDef::new(ParticipantClash::WasSeen)
                .boolean()
                .not_null()
                .default(true).to_owned(),
        ];
        match manager.get_database_backend() {
            DbBackend::Sqlite => {
                for mut column_def in column_defs {
                    manager
                        .alter_table(TableAlterStatement::new().table(ParticipantClash::Table).add_column(&mut column_def).to_owned())
                        .await?;
                }
            }
            _ => {
                let mut stmt = TableAlterStatement::new();
                stmt.table(ParticipantClash::Table);

                for mut column_def in column_defs {
                    stmt.add_column(&mut column_def);
                }
                
                manager.alter_table(stmt.to_owned()).await?;
            }
        }
        manager
        .alter_table(TableAlterStatement::new().table(Tournament::Table).add_column(
            ColumnDef::new(Tournament::AllowSelfDeclaredClashes)
                .boolean()
                .not_null()
                .default(false),
        ).to_owned()).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        todo!();
    }
}

