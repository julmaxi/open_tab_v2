use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Tournament {
    Table,
    Uuid,
}

#[derive(Iden)]
enum TournamentEntity {
    Table,
    Uuid,
    EntityType,
    TournamentId,
    IsDeleted,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TournamentEntity::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TournamentEntity::Uuid)
                            .uuid()
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(TournamentEntity::EntityType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TournamentEntity::TournamentId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TournamentEntity::IsDeleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tournament_entity_tournament_id")
                            .from(TournamentEntity::Table, TournamentEntity::TournamentId)
                            .to(Tournament::Table, Tournament::Uuid)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .primary_key(
                        Index::create()
                            .col(TournamentEntity::Uuid)
                            .primary(),
                    )
                    .to_owned(),
            )
            .await?;

        manager.create_index(
            Index::create()
                .table(TournamentEntity::Table)
                .name("idx_tournament_entity_tournament_id")
                .col(TournamentEntity::TournamentId)
                .to_owned(),
        ).await?;

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

#[derive(DeriveIden)]
enum Post {
    Table,
    Id,
    Title,
    Text,
}
