use std::{error::Error, collections::HashMap, default};

use migration::MigratorTrait;
use sea_orm::{DbErr, Database, Statement, ActiveValue};
use open_tab_entities::{domain::{participant::{Participant, Speaker, Adjudicator, ParticipantRole, ParticipantInstitution}, ballot::{Ballot, BallotTeam, TeamScore, self, Speech, SpeakerScore}, TournamentEntity, tournament::{Tournament, self}, round::TournamentRound, debate::TournamentDebate, tournament_institution::TournamentInstitution}, EntityGroups, Entity, schema::tournament_log};
use sea_orm::prelude::*;


pub async fn set_up_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();

    let r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    Ok(db)
}

fn make_changeset() -> (EntityGroups, Ballot) {
    let mut changeset = EntityGroups::new();
    changeset.add(Entity::Tournament(Tournament { uuid: Uuid::from_u128(10) }));
    changeset.add(Entity::TournamentRound(TournamentRound { uuid: Uuid::from_u128(20), tournament_id: Uuid::from_u128(10), index: 0, draw_type: None }));
    changeset.add(Entity::TournamentDebate(TournamentDebate { uuid: Uuid::from_u128(30), round_id: Uuid::from_u128(20), index: 0, current_ballot_uuid: Uuid::from_u128(100) }));
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=401).map(|u| Uuid::from_u128(u as u128)).collect(),
        government: BallotTeam {
            team: Some(Uuid::from_u128(200)),
            scores: HashMap::from_iter(
                vec![(Uuid::from_u128(401), TeamScore::Aggregate(140))].into_iter()
            ),
            ..Default::default()
        },
        speeches: vec![
            Speech { speaker: Some(Uuid::from_u128(402)), role: ballot::SpeechRole::Government, position: 0, scores: HashMap::from_iter(
                vec![(Uuid::from_u128(401), SpeakerScore::Aggregate(54))]
            )
        }],
        ..Default::default()
    };

    changeset.add(Entity::Ballot(
        ballot.clone()
    ));
    changeset.add(Entity::TournamentInstitution(
        TournamentInstitution {
            uuid: Uuid::from_u128(500),
            name: "Testclub".into(),
            tournament_id: Uuid::from_u128(10),
        }
    ));
    changeset.add(Entity::Participant(
        Participant {
            uuid: Uuid::from_u128(401),
            name: "Judge 1".into(),
            tournament_id: Uuid::from_u128(10),
            role: ParticipantRole::Adjudicator(Adjudicator { ..Default::default() }),
            institutions: vec![
                ParticipantInstitution { uuid: Uuid::from_u128(500), clash_severity: 2 }
            ]
        }
    ));
    changeset.add(Entity::Team(
        open_tab_entities::domain::team::Team {
            uuid: Uuid::from_u128(200),
            name: "Team 1".into(),
            tournament_id: Uuid::from_u128(10),
        }
    ));
    changeset.add(Entity::Participant(
        Participant {
            uuid: Uuid::from_u128(402),
            name: "Speaker 1".into(),
            tournament_id: Uuid::from_u128(10),
            role: ParticipantRole::Speaker(Speaker { team_id: Some(Uuid::from_u128(200)), ..Default::default() }),
            institutions: vec![]
        }
    ));

    changeset.add(
        Entity::ParticipantClash(
            open_tab_entities::domain::participant_clash::ParticipantClash {
                uuid: Uuid::from_u128(600),
                declaring_participant_id: Uuid::from_u128(401),
                target_participant_id: Uuid::from_u128(402),
                severity: 2
            }
        )
    );

    return (changeset, ballot);
}

#[tokio::test]
async fn test_save_full_tournament() -> Result<(), Box<dyn Error>> {
    let db = set_up_db().await?;

    let (changeset, ballot) = make_changeset();
    changeset.save_all(&db).await?;
    
    let saved_ballot = Ballot::get_many(&db, vec![Uuid::from_u128(100)]).await?;

    assert_eq!(saved_ballot.len(), 1);
    assert_eq!(saved_ballot[0], ballot);

    Ok(())
}

#[tokio::test]
async fn test_save_full_tournament_updates_log() -> Result<(), Box<dyn Error>> {
    let db = set_up_db().await?;

    let (changeset, _) = make_changeset();
    changeset.save_all(&db).await?;
    changeset.save_log_with_tournament_id(&db, Uuid::from_u128(10)).await?;
    
    let logs = tournament_log::Entity::find()
        .all(&db)
        .await?;

    assert_eq!(logs.len(), 9);

    Ok(())
}

#[tokio::test]
async fn test_versioned_save_preserves_uuids() -> Result<(), Box<dyn Error>> {
    let db = set_up_db().await?;

    let mut changeset = EntityGroups::new();
    changeset.add_versioned(Entity::Tournament(Tournament { uuid: Uuid::from_u128(10) }), Uuid::from_u128(10));
    changeset.add_versioned(Entity::TournamentRound(TournamentRound { uuid: Uuid::from_u128(20), tournament_id: Uuid::from_u128(10), index: 0, draw_type: None }), Uuid::from_u128(11));

    changeset.save_all(&db).await?;
    changeset.save_log_with_tournament_id(&db, Uuid::from_u128(10)).await?;
    
    let logs = tournament_log::Entity::find()
        .all(&db)
        .await?;

    assert_eq!(logs.len(), 2);
    assert!(logs.iter().any(
        |log| {
            log.uuid == Uuid::from_u128(10) && log.target_uuid == Uuid::from_u128(10)
        }
    ));
    assert!(logs.iter().any(
        |log| {
            log.uuid == Uuid::from_u128(11) && log.target_uuid == Uuid::from_u128(20)
        }
    ));

    Ok(())
}