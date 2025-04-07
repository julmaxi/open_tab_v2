use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create WellKnownInstitutions table
        manager
            .create_table(
                Table::create()
                    .table(WellKnownInstitution::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WellKnownInstitution::Uuid)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WellKnownInstitution::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WellKnownInstitution::ShortName)
                            .string()
                            .unique_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WellKnownInstitution::TinyImage)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WellKnownInstitution::HeaderImage)
                            .uuid()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create InstitutionAlias table
        manager
            .create_table(
                Table::create()
                    .table(InstitutionAlias::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InstitutionAlias::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InstitutionAlias::Institution)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InstitutionAlias::Alias)
                            .string()
                            .unique_key()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(InstitutionAlias::Table, InstitutionAlias::Institution)
                            .to(WellKnownInstitution::Table, WellKnownInstitution::Uuid),
                    )
                    .to_owned(),
            )
            .await?;

        // Create UserAssociatedInstitution table
        manager
            .create_table(
                Table::create()
                    .table(UserAssociatedInstitution::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserAssociatedInstitution::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserAssociatedInstitution::InstitutionId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserAssociatedInstitution::StartOfAssociation)
                            .date()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserAssociatedInstitution::EndOfAssociation)
                            .date()
                            .null(),
                    )
                    .primary_key(
                        Index::create()
                            .name("pk_user_associated_institution")
                            .col(UserAssociatedInstitution::UserId)
                            .col(UserAssociatedInstitution::InstitutionId)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserAssociatedInstitution::Table, UserAssociatedInstitution::InstitutionId)
                            .to(WellKnownInstitution::Table, WellKnownInstitution::Uuid),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserAssociatedInstitution::Table, UserAssociatedInstitution::UserId)
                            .to(User::Table, User::Uuid),
                    )
                    .to_owned(),
            )
            .await?;

        // Create Asset table
        manager
            .create_table(
                Table::create()
                    .table(Asset::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Asset::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Asset::Hash)
                            .binary_len(4)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager.alter_table(
            Table::alter()
                .table(TournamentInstitution::Table)
                .add_column(
                    ColumnDef::new(TournamentInstitution::OfficialIdentifier)
                        .string()
                )
                .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order of creation
        manager
            .drop_table(Table::drop().table(Asset::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserAssociatedInstitution::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(InstitutionAlias::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(WellKnownInstitution::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WellKnownInstitution {
    Table,
    Uuid,
    Name,
    ShortName,
    TinyImage,
    HeaderImage,
}

#[derive(DeriveIden)]
enum InstitutionAlias {
    Table,
    Id,
    Institution,
    Alias,
}

#[derive(DeriveIden)]
enum UserAssociatedInstitution {
    Table,
    UserId,
    InstitutionId,
    StartOfAssociation,
    EndOfAssociation,
}

#[derive(DeriveIden)]
enum Asset {
    Table,
    Id,
    Hash,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Uuid,
}

#[derive(DeriveIden)]
enum TournamentInstitution {
    Table,
    OfficialIdentifier
}