use sea_orm_migration::{prelude::*, manager};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000001_create_table"
    }
}


#[derive(Iden)]
pub enum Debate {
    Table,
    Uuid,
    Index,
    BallotId
}


#[derive(Iden)]
pub enum Ballot {
    Table,
    Uuid,
}

#[derive(Iden)]
pub enum BallotTeam {
    Table,
    BallotId,
    TeamId,
    Role
}


/*
#[derive(Iden)]
pub enum AdjudicatorTeamScore {
    Table,
    BallotTeamId,
    AdjudicatorId,
    ManualTotalScore,
}
 */
#[derive(Iden)]
pub enum AdjudicatorTeamScore {
    Table,
    BallotId,
    RoleId,
    AdjudicatorId,
    ManualTotalScore,
}


#[derive(Iden)]
pub enum BallotSpeech {
    Table,
    Uuid,
    BallotId,
    SpeakerId,
    Position,
    Role
}



#[derive(Iden)]
pub enum AdjudicatorSpeechScore {
    Table,
    AdjudicatorId,
    BallotId,
    BallotSpeechId,
    ManualTotalScore
}


#[derive(Iden)]
pub enum BallotAdjudicator {
    Table,
    BallotId,
    AdjudicatorId,
    Position,
    Role
}

#[derive(Iden)]
pub enum Adjudicator {
    Table,
    Uuid,
    Name
}


#[derive(Iden)]
pub enum Speaker {
    Table,
    Uuid,
    Name,
    TeamUuid
}


#[derive(Iden)]
pub enum Team {
    Table,
    Uuid,
    Name
}


#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        
        manager
        .create_table(
            sea_query::Table::create()
                .table(Adjudicator::Table)
                .if_not_exists()
                .col(ColumnDef::new(Adjudicator::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(Adjudicator::Name).string())
                .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(Team::Table)
                .if_not_exists()
                .col(ColumnDef::new(Team::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(Team::Name).string().not_null())
                .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(Speaker::Table)
                .if_not_exists()
                .col(ColumnDef::new(Speaker::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(Speaker::Name).string())
                .col(ColumnDef::new(Speaker::TeamUuid).uuid())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-speaker-team")
                        .from_tbl(Speaker::Table)
                        .from_col(Speaker::TeamUuid)
                        .to_tbl(Team::Table)
                        .to_col(Team::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-speaker-team-id")
            .table(Speaker::Table)
            .col(Speaker::TeamUuid)
            .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(Ballot::Table)
                .if_not_exists()
                .col(ColumnDef::new(Ballot::Uuid).uuid().not_null().primary_key())
                .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(Debate::Table)
                .if_not_exists()
                .col(ColumnDef::new(Debate::Index).big_integer().not_null())
                .col(ColumnDef::new(Debate::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(Debate::BallotId).uuid())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-debate-ballot")
                        .from_tbl(Debate::Table)
                        .from_col(Debate::BallotId)
                        .to_tbl(Ballot::Table)
                        .to_col(Ballot::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-debate-ballot-id")
            .table(Debate::Table)
            .col(Debate::BallotId)
            .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(BallotAdjudicator::Table)
                .if_not_exists()
                .col(ColumnDef::new(BallotAdjudicator::BallotId).uuid().not_null())
                .col(ColumnDef::new(BallotAdjudicator::AdjudicatorId).uuid().not_null())
                .col(ColumnDef::new(BallotAdjudicator::Position).integer().not_null())
                .col(ColumnDef::new(BallotAdjudicator::Role).string_len(1).not_null())
                .primary_key(
                    Index::create()
                        .name("pk-ballot_adjudicator")
                        .col(BallotAdjudicator::BallotId)
                        .col(BallotAdjudicator::AdjudicatorId)
                        .primary(),
                )    
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-adjudicator-adjudicator_ballot")
                        .from_tbl(BallotAdjudicator::Table)
                        .from_col(BallotAdjudicator::AdjudicatorId)
                        .to_tbl(Adjudicator::Table)
                        .to_col(Adjudicator::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-ballot-adjudicator_ballot")
                        .from_tbl(BallotAdjudicator::Table)
                        .from_col(BallotAdjudicator::BallotId)
                        .to_tbl(Ballot::Table)
                        .to_col(Ballot::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-ballot_adjudicator-ballot_id")
            .table(BallotAdjudicator::Table)
            .col(BallotAdjudicator::AdjudicatorId)
            .to_owned()
        ).await?;
        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-ballot_adjudicator-adjudicator_id")
            .table(BallotAdjudicator::Table)
            .col(BallotAdjudicator::AdjudicatorId)
            .to_owned()
        ).await?;


        manager
        .create_table(
            sea_query::Table::create()
                .table(BallotTeam::Table)
                .if_not_exists()
                .col(ColumnDef::new(BallotTeam::BallotId).uuid().not_null())
                .col(ColumnDef::new(BallotTeam::TeamId).uuid())
                .col(ColumnDef::new(BallotTeam::Role).string_len(1).not_null())
                .primary_key(Index::create().col(BallotTeam::BallotId).col(BallotTeam::Role).primary())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-team-ballot")
                        .from_tbl(BallotTeam::Table)
                        .from_col(BallotTeam::BallotId)
                        .to_tbl(Ballot::Table)
                        .to_col(Ballot::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-ballot_team-team_id")
            .table(BallotTeam::Table)
            .col(BallotTeam::TeamId)
            .to_owned()
        ).await?;
        
        manager
        .create_table(
            sea_query::Table::create()
                .table(AdjudicatorTeamScore::Table)
                .if_not_exists()
                .col(ColumnDef::new(AdjudicatorTeamScore::AdjudicatorId).uuid().not_null())
                .col(ColumnDef::new(AdjudicatorTeamScore::BallotId).uuid().not_null())
                .col(ColumnDef::new(AdjudicatorTeamScore::RoleId).string_len(1).not_null())
                .col(ColumnDef::new(AdjudicatorTeamScore::ManualTotalScore).integer())
                .primary_key(
                    Index::create()
                        .name("pk-adjudicator_team_score")
                        .col(AdjudicatorTeamScore::AdjudicatorId)
                        .col(AdjudicatorTeamScore::BallotId)
                        .col(AdjudicatorTeamScore::RoleId)
                        .primary(),
                )    
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-team-team_score")
                        .from_tbl(AdjudicatorTeamScore::Table)
                        .from_col(AdjudicatorTeamScore::BallotId)
                        .from_col(AdjudicatorTeamScore::RoleId)
                        .to_tbl(BallotTeam::Table)
                        .to_col(BallotTeam::BallotId)
                        .to_col(BallotTeam::Role)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-ballot-team_score")
                        .from_tbl(AdjudicatorTeamScore::Table)
                        .from_col(AdjudicatorTeamScore::BallotId)
                        .to_tbl(Ballot::Table)
                        .to_col(Ballot::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-adjudicator-team_score")
                        .from_tbl(AdjudicatorTeamScore::Table)
                        .from_col(AdjudicatorTeamScore::AdjudicatorId)
                        .from_col(AdjudicatorTeamScore::BallotId)
                        .to_tbl(BallotAdjudicator::Table)
                        .to_col(BallotAdjudicator::AdjudicatorId)
                        .to_col(BallotAdjudicator::BallotId)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                    )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-adjudicator_team_score-ballot_id")
            .table(AdjudicatorTeamScore::Table)
            .col(AdjudicatorTeamScore::BallotId)
            .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(BallotSpeech::Table)
                .if_not_exists()
                .col(ColumnDef::new(BallotSpeech::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(BallotSpeech::BallotId).uuid().not_null())
                .col(ColumnDef::new(BallotSpeech::Position).integer().not_null())
                .col(ColumnDef::new(BallotSpeech::Role).string_len(1).not_null())
                .col(ColumnDef::new(BallotSpeech::SpeakerId).uuid())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-speech-ballot")
                        .from_tbl(BallotSpeech::Table)
                        .from_col(BallotSpeech::BallotId)
                        .to_tbl(Ballot::Table)
                        .to_col(Ballot::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-speech-speaker")
                        .from_tbl(BallotSpeech::Table)
                        .from_col(BallotSpeech::SpeakerId)
                        .to_tbl(Speaker::Table)
                        .to_col(Speaker::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-speech-ballot_id")
            .table(BallotSpeech::Table)
            .col(BallotSpeech::BallotId)
            .to_owned()
        ).await?;
        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-speech-speaker_id")
            .table(BallotSpeech::Table)
            .col(BallotSpeech::SpeakerId)
            .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(AdjudicatorSpeechScore::Table)
                .if_not_exists()
                .col(ColumnDef::new(AdjudicatorSpeechScore::AdjudicatorId).uuid().not_null())
                .col(ColumnDef::new(AdjudicatorSpeechScore::BallotSpeechId).uuid().not_null())
                .col(ColumnDef::new(AdjudicatorSpeechScore::BallotId).uuid().not_null())
                .col(ColumnDef::new(AdjudicatorSpeechScore::ManualTotalScore).integer())
                .primary_key(
                    Index::create()
                        .name("pk-adjudicator_speech_score")
                        .col(AdjudicatorSpeechScore::AdjudicatorId)
                        .col(AdjudicatorSpeechScore::BallotSpeechId)
                        .primary(),
                )    
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-adjudicator_speech_score-speech")
                        .from_tbl(AdjudicatorSpeechScore::Table)
                        .from_col(AdjudicatorSpeechScore::BallotSpeechId)
                        .to_tbl(BallotSpeech::Table)
                        .to_col(BallotSpeech::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-adjudicator_speech_score-adjudicator")
                        .from_tbl(AdjudicatorSpeechScore::Table)
                        .from_col(AdjudicatorSpeechScore::AdjudicatorId)
                        .from_col(AdjudicatorSpeechScore::BallotId)
                        .to_tbl(BallotAdjudicator::Table)
                        .to_col(BallotAdjudicator::AdjudicatorId)
                        .to_col(BallotAdjudicator::BallotId)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-adjudicator_speech_score-ballot")
                        .from_tbl(AdjudicatorSpeechScore::Table)
                        .from_col(AdjudicatorSpeechScore::BallotId)
                        .to_tbl(Ballot::Table)
                        .to_col(Ballot::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-speech_score-adjudicator_id")
            .table(AdjudicatorSpeechScore::Table)
            .col(AdjudicatorSpeechScore::AdjudicatorId)
            .to_owned()
        ).await?;
        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-speech_score-ballot_id")
            .table(AdjudicatorSpeechScore::Table)
            .col(AdjudicatorSpeechScore::BallotId)
            .to_owned()
        ).await?;
        Result::Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        todo!()
    }
}