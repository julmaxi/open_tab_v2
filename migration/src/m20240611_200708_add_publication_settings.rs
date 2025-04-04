use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum PublishedTournament {
    Table,
    Uuid,
    TournamentId,
    PublicName,
    ImageData,
    ImageType,
    ListPublicly,
    ShowMotions,
    ShowDraws,
    ShowTab,
    ShowParticipants,
    StartDate,
    EndDate,
    Location
}

#[derive(Iden)]
enum Tournament {
    Table,
    Uuid
}


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
        .create_table(
            Table::create()
                .table(PublishedTournament::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(PublishedTournament::Uuid)
                        .uuid()
                        .not_null()
                        .primary_key()
                )
                .col(
                    ColumnDef::new(PublishedTournament::TournamentId)
                        .uuid()
                 )
                 .col(
                    ColumnDef::new(PublishedTournament::PublicName)
                        .string()
                        .not_null()
                 )
                .col(
                    ColumnDef::new(PublishedTournament::ImageData)
                        .blob()
                )
                .col(
                    ColumnDef::new(PublishedTournament::ImageType)
                        .string()
                )
                .col(
                    ColumnDef::new(PublishedTournament::ListPublicly)
                        .boolean()
                        .default(false)
                        .not_null()
                )
                .col(
                    ColumnDef::new(PublishedTournament::ShowMotions)
                        .boolean()
                        .default(false)
                        .not_null()
                )
                .col(
                    ColumnDef::new(PublishedTournament::ShowDraws)
                        .boolean()
                        .default(false)
                        .not_null()
                )
                .col(
                    ColumnDef::new(PublishedTournament::ShowTab)
                        .boolean()
                        .default(false)
                        .not_null()
                )
                .col(
                    ColumnDef::new(PublishedTournament::ShowParticipants)
                        .boolean()
                        .default(false)
                        .not_null()
                )
                .col(
                    ColumnDef::new(PublishedTournament::StartDate)
                        .date_time()
                )
                .col(
                    ColumnDef::new(PublishedTournament::EndDate)
                        .date_time()
                )
                .col(
                    ColumnDef::new(PublishedTournament::Location)
                        .string()
                )
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_published_tournament_tournament_id")
                    .from(PublishedTournament::Table, PublishedTournament::TournamentId)
                    .to(Tournament::Table, Tournament::Uuid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .table(PublishedTournament::Table)
                .name("idx-published_tournament_tournament_id")
                .col(PublishedTournament::TournamentId)
                .unique()
                .to_owned()
        ).await?;
        
        manager.create_index(
            Index::create()
                .table(PublishedTournament::Table)
                .name("published_tournament_start_date")
                .col(PublishedTournament::StartDate)
                .to_owned()
        ).await?;

        manager.create_index(
            Index::create()
                .table(PublishedTournament::Table)
                .name("published_tournament_end_date")
                .col(PublishedTournament::EndDate)
                .to_owned()
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        todo!();
    }
}
