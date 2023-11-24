

use migration::MigratorTrait;
use sea_orm::{Database, Statement, ActiveValue, TransactionTrait};
use open_tab_entities::domain::{participant::{Participant, Speaker, Adjudicator, ParticipantRole, ParticipantInstitution}, TournamentEntity, entity::LoadEntity};
use sea_orm::prelude::*;


pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, anyhow::Error> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        let tournament : open_tab_entities::schema::tournament::ActiveModel = open_tab_entities::schema::tournament::Model {
            uuid: Uuid::from_u128(1),
            annoucements_password: Some("test".into()),
            name: "Test Tournament".into(),
            feedback_release_time: None,
        }.into();
        tournament.insert(&db).await?;

        let round_ = open_tab_entities::domain::round::TournamentRound {
            uuid: Uuid::from_u128(10),
            tournament_id: Uuid::from_u128(1),
            ..Default::default()
        };
        round_.save(&db, true).await?;
        open_tab_entities::schema::tournament_institution::Entity::insert(
            open_tab_entities::schema::tournament_institution::ActiveModel {
                tournament_id: ActiveValue::Set(Uuid::from_u128(1)),
                uuid: ActiveValue::Set(Uuid::from_u128(500)),
                name: ActiveValue::Set("Testclub".into()),
            }
        ).exec(&db).await?;

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
                chair_skill: ActiveValue::Set(26),
                panel_skill: ActiveValue::Set(29),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(401)),
                chair_skill: ActiveValue::Set(1),
                panel_skill: ActiveValue::Set(1),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(402)),
                chair_skill: ActiveValue::Set(1),
                panel_skill: ActiveValue::Set(1),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(403)),
                chair_skill: ActiveValue::Set(1),
                panel_skill: ActiveValue::Set(1),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(404)),
                chair_skill: ActiveValue::Set(1),
                panel_skill: ActiveValue::Set(1),
                ..Default::default()
            },
            open_tab_entities::schema::adjudicator::ActiveModel {
                uuid: ActiveValue::Set(Uuid::from_u128(405)),
                chair_skill: ActiveValue::Set(1),
                panel_skill: ActiveValue::Set(1),
                ..Default::default()
            }
        ]).exec(&db).await?;
    }
    Ok(db)
}

async fn test_participant_roundtrip_in_db<C>(db: &C, participant: Participant, as_insert: bool) -> Result<(), anyhow::Error> where C: ConnectionTrait {
    participant.save(db, as_insert).await?;

    let mut saved_participant = Participant::get_many(
        db,
        vec![participant.uuid]
    ).await?;

    assert_eq!(saved_participant.len(), 1);
    let saved_participant = saved_participant.pop().unwrap();
    assert_eq!(participant, saved_participant);

    Ok(())
}

async fn test_participant_roundtrip(participant: Participant, as_insert: bool) -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    test_participant_roundtrip_in_db(&db, participant, as_insert).await?;
    Ok(())
}

#[tokio::test]
async fn test_speaker_roundtrip() -> Result<(), anyhow::Error> {
    test_participant_roundtrip(Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Speaker(Speaker {
            team_id: Some(Uuid::from_u128(200))
        }),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![
            ParticipantInstitution {
                uuid: Uuid::from_u128(500),
                clash_severity: 20
            }
        ],
        registration_key: None,
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_adjudicator_roundtrip() -> Result<(), anyhow::Error> {
    test_participant_roundtrip(Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {
            ..Default::default()
        }),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![],
        registration_key: None,
    }, true).await?;

    Ok(())
}


#[tokio::test]
async fn test_save_adjudicator_round_availability_override() -> Result<(), anyhow::Error> {
    let participant = Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {
            unavailable_rounds: vec![Uuid::from_u128(10)],
            ..Default::default()
        }),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![],
        registration_key: None,
    };
    test_participant_roundtrip(participant, true).await?;

    Ok(())
}


#[tokio::test]
async fn test_remove_adjudicator_round_availability_override() -> Result<(), anyhow::Error> {
    let mut participant = Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(Adjudicator {
            unavailable_rounds: vec![Uuid::from_u128(10)],
            ..Default::default()
        }),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![],
        registration_key: None,
    };
    let db = set_up_db(true).await?;
    let t = db.begin().await?;
    participant.save(&t, true).await?;
    t.commit().await?;

    match &mut participant.role {
        ParticipantRole::Adjudicator(a) => {
            a.unavailable_rounds = vec![];
        },
        _ => panic!("Not an adjudicator")
    }
    participant.save(&db, false).await?;    

    let participant = Participant::get(&db, Uuid::from_u128(440)).await?;
    match participant.role {
        ParticipantRole::Adjudicator(a) => {
            assert_eq!(a.unavailable_rounds, vec![]);
        },
        _ => panic!("Not an adjudicator")
    }
    Ok(())
}


#[tokio::test]
async fn test_make_speaker_into_adjudicator() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut participant = Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: ParticipantRole::Speaker(Speaker {
            team_id: Some(Uuid::from_u128(200))
        }),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![],
        registration_key: None,
    };

    participant.save(&db, true).await?;

    participant.role = ParticipantRole::Adjudicator(Adjudicator {..Default::default() });

    test_participant_roundtrip_in_db(&db, participant, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_make_adjudicator_into_speaker() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut participant = Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![],
        registration_key: None,
    };

    participant.save(&db, true).await?;

    participant.role = ParticipantRole::Speaker(Speaker {
        team_id: Some(Uuid::from_u128(200))
    });

    test_participant_roundtrip_in_db(&db, participant, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_change_participant_name() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut participant = Participant {
        uuid: Uuid::from_u128(440),
        name: "Test".into(),
        role: ParticipantRole::Adjudicator(Adjudicator {..Default::default() }),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![],
        registration_key: None,
    };

    participant.save(&db, true).await?;

    participant.name = "New Name".into();

    test_participant_roundtrip_in_db(&db, participant, false).await?;

    Ok(())
}