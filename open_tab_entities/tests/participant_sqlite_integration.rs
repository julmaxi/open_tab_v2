use std::error::Error;

use migration::MigratorTrait;
use sea_orm::{DbErr, Database, Statement, ActiveValue};
use open_tab_entities::domain::{participant::{Participant, Speaker, Adjudicator, ParticipantRole}, ballot::Ballot, TournamentEntity};
use sea_orm::prelude::*;


pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        let a : open_tab_entities::schema::tournament::ActiveModel = open_tab_entities::schema::tournament::Model {
            uuid: Uuid::from_u128(1),
        }.into();
        a.insert(&db).await?;
         open_tab_entities::schema::team::Entity::insert_many(vec![
            open_tab_entities::schema::team::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(200)),
                name: ActiveValue::Set("Team 1".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            },
            open_tab_entities::schema::team::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(201)),
                name: ActiveValue::Set("Team 2".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            }
        ]).exec(&db).await?;

        open_tab_entities::schema::participant::Entity::insert_many(vec![
            open_tab_entities::schema::participant::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(400)),
                name: ActiveValue::Set("Judge 1".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            },
            open_tab_entities::schema::participant::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(401)),
                name: ActiveValue::Set("Judge 2".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            },
            open_tab_entities::schema::participant::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(402)),
                name: ActiveValue::Set("Judge 3".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            },
            open_tab_entities::schema::participant::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(403)),
                name: ActiveValue::Set("Judge 4".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            },
            open_tab_entities::schema::participant::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(404)),
                name: ActiveValue::Set("Judge 5".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            },
            open_tab_entities::schema::participant::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(405)),
                name: ActiveValue::Set("Judge 6".into()),
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                ..Default::default()
            }
        ]).exec(&db).await?;

        open_tab_entities::schema::adjudicator::Entity::insert_many(vec![
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(400)),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(401)),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(402)),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(403)),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(404)),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(405)),
                ..Default::default()
            }
        ]).exec(&db).await?;
    }
    Ok(db)
}

async fn test_participant_roundtrip_in_db<C>(db: &C, participant: Participant, as_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
    participant.save(db, as_insert).await?;

    let mut saved_ballot = Participant::get_many(
        db,
        vec![participant.uuid]
    ).await?;

    assert_eq!(saved_ballot.len(), 1);
    let saved_ballot = saved_ballot.pop().unwrap();
    assert_eq!(participant, saved_ballot);

    Ok(())
}

async fn test_participant_roundtrip(participant: Participant, as_insert: bool) -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    test_participant_roundtrip_in_db(&db, participant, as_insert).await?;
    Ok(())
}

#[tokio::test]
async fn test_speaker_roundtrip() -> Result<(), Box<dyn Error>> {
    test_participant_roundtrip(Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Speaker(Speaker {
            team_id: Some(Uuid::from_u128(200))
        }),
        tournament_id: Uuid::from_u128(1),
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_adjudicator_roundtrip() -> Result<(), Box<dyn Error>> {
    test_participant_roundtrip(Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {
        }),
        tournament_id: Uuid::from_u128(1),
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_make_speaker_into_adjudicator() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let mut participant = Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: ParticipantRole::Speaker(Speaker {
            team_id: Some(Uuid::from_u128(200))
        }),
        tournament_id: Uuid::from_u128(1),
    };

    participant.save(&db, true).await?;

    participant.role = ParticipantRole::Adjudicator(Adjudicator{});

    test_participant_roundtrip_in_db(&db, participant, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_make_adjudicator_into_speaker() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let mut participant = Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: ParticipantRole::Adjudicator(Adjudicator{}),
        tournament_id: Uuid::from_u128(1),
    };

    participant.save(&db, true).await?;

    participant.role = ParticipantRole::Speaker(Speaker {
        team_id: Some(Uuid::from_u128(200))
    });

    test_participant_roundtrip_in_db(&db, participant, false).await?;

    Ok(())
}