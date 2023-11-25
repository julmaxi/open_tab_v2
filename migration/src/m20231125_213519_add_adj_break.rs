use sea_orm_migration::{prelude::*, seaql_migrations::PrimaryKey};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
pub enum TournamentBreakAdjudicator {
    Table,
    TournamentBreakId,
    AdjudicatorId,
}


#[derive(DeriveIden)]
pub enum TournamentBreak {
    Table,
    Uuid,
}

#[derive(DeriveIden)]
pub enum Adjudicator {
    Table,
    Uuid,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
        .create_table(
            Table::create()
                .table(TournamentBreakAdjudicator::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(TournamentBreakAdjudicator::TournamentBreakId)
                        .uuid()
                        .not_null()
                )
                .col(
                    ColumnDef::new(TournamentBreakAdjudicator::AdjudicatorId)
                        .uuid()
                        .not_null()
                )
                .primary_key(
                    Index::create()
                        .col(TournamentBreakAdjudicator::TournamentBreakId)
                        .col(TournamentBreakAdjudicator::AdjudicatorId)
                        .primary(),
                )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_break-adjudicator_break_id")
                    .from(TournamentBreakAdjudicator::Table, TournamentBreakAdjudicator::TournamentBreakId)
                    .to(TournamentBreak::Table, TournamentBreak::Uuid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_break-adjudicator_adjudicator_id")
                    .from(TournamentBreakAdjudicator::Table, TournamentBreakAdjudicator::AdjudicatorId)
                    .to(Adjudicator::Table, Adjudicator::Uuid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TournamentBreakAdjudicator::Table).to_owned())
            .await
    }
}
