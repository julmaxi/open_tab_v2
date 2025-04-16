use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AwardSeries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AwardSeries::Uuid)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AwardSeries::ShortName)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(AwardSeries::Name).string().not_null())
                    .col(ColumnDef::new(AwardSeries::Prestige).integer().not_null())
                    .col(ColumnDef::new(AwardSeries::Image).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-award_series-image")
                            .from(AwardSeries::Table, AwardSeries::Image)
                            .to(Asset::Table, Asset::Uuid)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(TournamentBreak::Table)
                    .add_column(
                        ColumnDef::new(TournamentBreak::AwardSeriesKey)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(TournamentPlanNode::Table)
                    .add_column(
                        ColumnDef::new(TournamentPlanNode::SuggestedAwardSeriesKey)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TournamentPlanNode::Table)
                    .drop_column(TournamentPlanNode::SuggestedAwardSeriesKey)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(TournamentBreak::Table)
                    .drop_column(TournamentBreak::AwardSeriesKey)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(AwardSeries::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AwardSeries {
    Table,
    Uuid,
    ShortName,
    Name,
    Prestige,
    Image,
}

#[derive(DeriveIden)]
enum TournamentBreak {
    Table,
    AwardSeriesKey,
}

#[derive(DeriveIden)]
enum TournamentPlanNode {
    Table,
    SuggestedAwardSeriesKey,
}

#[derive(DeriveIden)]
enum Asset {
    Table,
    Uuid
}