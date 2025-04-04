use sea_orm_migration::{prelude::*, sea_orm::{DbBackend, Statement}};

#[derive(DeriveMigrationName)]
pub struct Migration;


#[derive(DeriveIden)]
enum Tournament {
    Table,
    Uuid
}

#[derive(DeriveIden)]
enum Team {
    Table,
    Uuid,
}

#[derive(DeriveIden)]
enum Participant {
    Table,
    Uuid,
    BreakCategoryId
}

#[derive(DeriveIden)]
pub enum TournamentBreakCategory {
    Table,
    Uuid,
    Name,
    TournamentId,
}

#[derive(DeriveIden)]
pub enum TournamentBreakEligibleCategory {
    Table,
    TournamentBreakCategoryId,
    TournamentPlanNodeId,
    Config,
}

#[derive(DeriveIden)]
pub enum TournamentBreak {
    Table,
    BreakAwardTitle,
    BreakAwardPrestige
}

#[derive(DeriveIden)]
pub enum TournamentPlanNode {
    Table,
    Uuid,
    SuggestedAwardTitle,
    SuggestedAwardPrestige,
    MaxBreakingAdjudicatorCount,
    IsOnlyAward
}


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(TournamentBreakCategory::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(TournamentBreakCategory::Uuid)
                        .uuid()
                        .not_null()
                        .primary_key()
                )
                .col(
                    ColumnDef::new(TournamentBreakCategory::Name)
                        .string_len(255)
                        .not_null()
                )
                .col(
                    ColumnDef::new(TournamentBreakCategory::TournamentId)
                        .uuid()
                        .not_null()
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-tournament-break-category-tournament-id")
                        .from(TournamentBreakCategory::Table, TournamentBreakCategory::TournamentId)
                        .to(Tournament::Table, Tournament::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned(),
        ).await?;

        manager.create_table(
            Table::create()
                .table(TournamentBreakEligibleCategory::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(TournamentBreakEligibleCategory::TournamentBreakCategoryId)
                        .uuid()
                        .not_null()
                )
                .col(
                    ColumnDef::new(TournamentBreakEligibleCategory::TournamentPlanNodeId)
                        .uuid()
                        .not_null()
                )
                .col(
                    ColumnDef::new(TournamentBreakEligibleCategory::Config)
                        .json()
                        .not_null()
                )
                .primary_key(
                    Index::create()
                        .name("pk-tournament-break-eligible-category")
                        .col(TournamentBreakEligibleCategory::TournamentBreakCategoryId)
                        .col(TournamentBreakEligibleCategory::TournamentPlanNodeId)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-tournament-break-eligible-category-tournament-break-category-id")
                        .from(TournamentBreakEligibleCategory::Table, TournamentBreakEligibleCategory::TournamentBreakCategoryId)
                        .to(TournamentBreakCategory::Table, TournamentBreakCategory::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-tournament-break-eligible-category-tournament-break-id")
                        .from(TournamentBreakEligibleCategory::Table, TournamentBreakEligibleCategory::TournamentPlanNodeId)
                        .to(TournamentPlanNode::Table, TournamentPlanNode::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(TournamentBreak::Table)
                .add_column(
                    ColumnDef::new(TournamentBreak::BreakAwardTitle)
                        .string_len(255)
                )
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(TournamentPlanNode::Table)
                .add_column(
                    ColumnDef::new(TournamentPlanNode::SuggestedAwardTitle)
                        .string_len(255)
                )
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(TournamentPlanNode::Table)
                .add_column(
                    ColumnDef::new(TournamentPlanNode::MaxBreakingAdjudicatorCount)
                        .integer()
                )
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(TournamentPlanNode::Table)
                .add_column(
                    ColumnDef::new(TournamentPlanNode::IsOnlyAward)
                        .boolean()
                        .default(false)
                        .not_null()
                )
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(TournamentBreak::Table)
                .add_column(
                    ColumnDef::new(TournamentBreak::BreakAwardPrestige)
                        .integer()
                )
                .to_owned(),
        ).await?;

        manager.alter_table(
            Table::alter()
                .table(TournamentPlanNode::Table)
                .add_column(
                    ColumnDef::new(TournamentPlanNode::SuggestedAwardPrestige)
                        .integer()
                )
                .to_owned(),
        ).await?;

        match manager.get_database_backend() {
            DbBackend::Sqlite => {
                manager.get_connection().execute_unprepared(
                    r"
                    ALTER TABLE participant
                    ADD COLUMN break_category_id UUID REFERENCES tournament_break_category(uuid) ON DELETE SET NULL ON UPDATE CASCADE;
                    "
                ).await?;
            }
            _ => {

                manager.alter_table(
                    Table::alter()
                        .table(Participant::Table)
                        .add_column(
                            ColumnDef::new(Participant::BreakCategoryId)
                                .uuid()
                        )
                        .to_owned(),
                ).await?;
        
                manager.alter_table(
                    Table::alter()
                        .table(Participant::Table)
                        .add_foreign_key(
                            TableForeignKey::new()
                     .name("fk-participant-break-category-id")
                     .from_tbl(Participant::Table)
                     .from_col(Participant::BreakCategoryId)
                     .to_tbl(TournamentBreakCategory::Table)
                     .to_col(TournamentBreakCategory::Uuid)
                     .on_delete(ForeignKeyAction::SetNull)
                     .on_update(ForeignKeyAction::SetNull)
                         )
                         .to_owned(),
                ).await?;
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();

    }
}
