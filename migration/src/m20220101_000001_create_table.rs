use sea_orm_migration::{prelude::*, manager};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000001_create_table"
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
enum TournamentDebate {
    Table,
    Uuid,
    RoundId,
    Index,
    BallotId
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

#[derive(Iden)]
enum Ballot {
    Table,
    Uuid,
}




#[derive(Iden)]
enum Participant {
    Table,
    Uuid,
    TournamentId,
    Name,
}


#[derive(Iden)]
pub enum Debate {
    Table,
    Uuid,
    Index,
    BallotId
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
    SpeechRole,
    SpeechPosition,
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
}


#[derive(Iden)]
pub enum Speaker {
    Table,
    Uuid,
    TeamId,
}


#[derive(Iden)]
pub enum Team {
    Table,
    Uuid,
    Name,
    TournamentId,
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

        manager.create_table(
            sea_query::Table::create()
                .table(TournamentRound::Table)
                .if_not_exists()
                .col(ColumnDef::new(TournamentRound::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(TournamentRound::TournamentId).uuid().not_null())
                .col(ColumnDef::new(TournamentRound::Index).unsigned().not_null())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-round-tournament")
                        .from_tbl(TournamentRound::Table)
                        .from_col(TournamentRound::TournamentId)
                        .to_tbl(Tournament::Table)
                        .to_col(Tournament::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned()
        ).await?;


        manager
        .create_table(
            sea_query::Table::create()
                .table(TournamentLog::Table)
                .if_not_exists()
                .col(ColumnDef::new(TournamentLog::Uuid).uuid().primary_key())
                .col(ColumnDef::new(TournamentLog::TournamentId).uuid().not_null())
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

        manager.create_table(
            sea_query::Table::create()
                .table(Participant::Table)
                .if_not_exists()
                .col(ColumnDef::new(Participant::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(Participant::TournamentId).uuid().not_null())
                .col(ColumnDef::new(Participant::Name).string().not_null())
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-participant_tournament-idx")
            .table(Participant::Table)
            .col(Participant::TournamentId)
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

        manager
        .create_table(
            sea_query::Table::create()
                .table(Adjudicator::Table)
                .if_not_exists()
                .col(ColumnDef::new(Adjudicator::Uuid).uuid().not_null().primary_key())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                    .name("fk-adjudicator-participant")
                    .from_tbl(Adjudicator::Table)
                    .from_col(Adjudicator::Uuid)
                    .to_tbl(Participant::Table)
                    .to_col(Participant::Uuid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)    
                )
                .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(Team::Table)
                .if_not_exists()
                .col(ColumnDef::new(Team::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(Team::Name).string().not_null())
                .col(ColumnDef::new(Team::TournamentId).uuid().not_null())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                    .name("fk-team-tournament")
                    .from_tbl(Team::Table)
                    .from_col(Team::TournamentId)
                    .to_tbl(Tournament::Table)
                    .to_col(Tournament::Uuid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)    
                )
                .to_owned()
        ).await?;

        manager
        .create_table(
            sea_query::Table::create()
                .table(Speaker::Table)
                .if_not_exists()
                .col(ColumnDef::new(Speaker::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(Speaker::TeamId).uuid())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-speaker-team")
                        .from_tbl(Speaker::Table)
                        .from_col(Speaker::TeamId)
                        .to_tbl(Team::Table)
                        .to_col(Team::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                    .name("fk-speaker-participant")
                    .from_tbl(Speaker::Table)
                    .from_col(Speaker::Uuid)
                    .to_tbl(Participant::Table)
                    .to_col(Participant::Uuid)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)    
                )
                .to_owned()
        ).await?;

        manager.create_index(
            IndexCreateStatement::new()
            .name("idx-speaker-team-id")
            .table(Speaker::Table)
            .col(Speaker::TeamId)
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

        manager.create_table(
            sea_query::Table::create()
                .table(TournamentDebate::Table)
                .if_not_exists()
                .col(ColumnDef::new(TournamentDebate::Uuid).uuid().not_null().primary_key())
                .col(ColumnDef::new(TournamentDebate::RoundId).uuid().not_null().not_null())
                .col(ColumnDef::new(TournamentDebate::Index).unsigned().not_null())
                .col(ColumnDef::new(TournamentDebate::BallotId).uuid().not_null())
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-debate-round")
                        .from_tbl(TournamentDebate::Table)
                        .from_col(TournamentDebate::RoundId)
                        .to_tbl(TournamentRound::Table)
                        .to_col(TournamentRound::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-debate-ballot")
                        .from_tbl(TournamentDebate::Table)
                        .from_col(TournamentDebate::BallotId)
                        .to_tbl(Ballot::Table)
                        .to_col(Ballot::Uuid)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade)
                )
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
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-ballot_team-team")
                        .from_tbl(BallotTeam::Table)
                        .from_col(BallotTeam::TeamId)
                        .to_tbl(Team::Table)
                        .to_col(Team::Uuid)
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
                .col(ColumnDef::new(BallotSpeech::BallotId).uuid().not_null())
                .col(ColumnDef::new(BallotSpeech::Position).integer().not_null())
                .col(ColumnDef::new(BallotSpeech::Role).string_len(1).not_null())
                .col(ColumnDef::new(BallotSpeech::SpeakerId).uuid())
                .primary_key(
                    Index::create()
                        .name("pk-speech")
                        .col(BallotSpeech::BallotId)
                        .col(BallotSpeech::Role)
                        .col(BallotSpeech::Position)
                        .primary(),
                )    
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
                .col(ColumnDef::new(AdjudicatorSpeechScore::BallotId).uuid().not_null())
                .col(ColumnDef::new(AdjudicatorSpeechScore::SpeechRole).string_len(1).not_null())
                .col(ColumnDef::new(AdjudicatorSpeechScore::SpeechPosition).integer().not_null())
                .col(ColumnDef::new(AdjudicatorSpeechScore::ManualTotalScore).integer())
                .primary_key(
                    Index::create()
                        .name("pk-adjudicator_speech_score")
                        .col(AdjudicatorSpeechScore::AdjudicatorId)
                        .col(AdjudicatorSpeechScore::BallotId)
                        .col(AdjudicatorSpeechScore::SpeechRole)
                        .col(AdjudicatorSpeechScore::SpeechPosition)
                        .primary(),
                )    
                .foreign_key(
                    ForeignKeyCreateStatement::new()
                        .name("fk-adjudicator_speech_score-speech")
                        .from_tbl(AdjudicatorSpeechScore::Table)
                        .from_col(AdjudicatorSpeechScore::BallotId)
                        .from_col(AdjudicatorSpeechScore::SpeechRole)
                        .from_col(AdjudicatorSpeechScore::SpeechPosition)
                        .to_tbl(BallotSpeech::Table)
                        .to_col(BallotSpeech::BallotId)
                        .to_col(BallotSpeech::Role)
                        .to_col(BallotSpeech::Position)
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