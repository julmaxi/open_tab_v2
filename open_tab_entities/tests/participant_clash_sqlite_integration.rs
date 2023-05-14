use std::error::Error;

use migration::MigratorTrait;
use sea_orm::{DbErr, Database, Statement, ActiveValue};
use open_tab_entities::domain::{participant::{Participant, Speaker, Adjudicator, ParticipantRole, ParticipantInstitution}, ballot::Ballot, TournamentEntity, participant_clash::ParticipantClash, entity::LoadEntity};
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
        for i in 0..2 {
            let tournament : open_tab_entities::schema::tournament::ActiveModel = open_tab_entities::schema::tournament::Model {
                uuid: Uuid::from_u128(1 + i),
            }.into();
            tournament.insert(&db).await?;

            open_tab_entities::schema::team::Entity::insert_many(vec![
                open_tab_entities::schema::team::ActiveModel {
                    uuid: ActiveValue::Set(Uuid::from_u128(200 + i)),
                    name: ActiveValue::Set("Team 1".into()),
                    tournament_id: ActiveValue::Set(Uuid::from_u128(1 + i)),
                    ..Default::default()
                },
            ]).exec(&db).await?;

            open_tab_entities::schema::participant::Entity::insert_many(vec![
                open_tab_entities::schema::participant::ActiveModel {
                    uuid: ActiveValue::Set(Uuid::from_u128(440 + i)),
                    name: ActiveValue::Set("Speaker 1".into()),
                    tournament_id: ActiveValue::Set(Uuid::from_u128(1 + i)),
                    ..Default::default()
                },
            ]).exec(&db).await?;

            open_tab_entities::schema::speaker::Entity::insert_many(vec![
                open_tab_entities::schema::speaker::ActiveModel {
                    uuid: ActiveValue::Set(Uuid::from_u128(440 + i)),
                    team_id: ActiveValue::Set(Some(Uuid::from_u128(200 + i))),
                    ..Default::default()
                },
            ]).exec(&db).await?;

            open_tab_entities::schema::participant::Entity::insert_many(vec![
                open_tab_entities::schema::participant::ActiveModel {
                    uuid: ActiveValue::Set(Uuid::from_u128(400 + i)),
                    name: ActiveValue::Set("Judge 1".into()),
                    tournament_id: ActiveValue::Set(Uuid::from_u128(1 + i)),
                    ..Default::default()
                },
            ]).exec(&db).await?;

            open_tab_entities::schema::adjudicator::Entity::insert_many(vec![
                open_tab_entities::schema::adjudicator::ActiveModel {
                    uuid: ActiveValue::Set(Uuid::from_u128(400 + i)),
                    chair_skill: ActiveValue::Set(26),
                    panel_skill: ActiveValue::Set(29),
                    ..Default::default()
                },
            ]).exec(&db).await?;
        }
    }
    Ok(db)
}

async fn test_clash_roundtrip_in_db<C>(db: &C, clash: Participant, as_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
    clash.save(db, as_insert).await?;

    let mut saved_participant = Participant::get_many(
        db,
        vec![clash.uuid]
    ).await?;

    assert_eq!(saved_participant.len(), 1);
    let saved_participant = saved_participant.pop().unwrap();
    assert_eq!(clash, saved_participant);

    Ok(())
}


#[tokio::test]
async fn test_get_all_clashes_in_tournament() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let clash = ParticipantClash {
        uuid: Uuid::from_u128(600),
        declaring_participant_id: Uuid::from_u128(440),
        target_participant_id: Uuid::from_u128(400),
        clash_severity: 20,
    };
    clash.save(&db, true).await?;

    let clash = ParticipantClash {
        uuid: Uuid::from_u128(601),
        declaring_participant_id: Uuid::from_u128(441),
        target_participant_id: Uuid::from_u128(401),
        clash_severity: 20,
    };
    clash.save(&db, true).await?;


    let clashes = ParticipantClash::get_all_in_tournament(&db, Uuid::from_u128(1)).await?;
    let clashes = ParticipantClash::get_all_in_tournament(&db, Uuid::from_u128(2)).await?;

    assert_eq!(clashes.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_ignore_inter_tournament_clashes() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let clash = ParticipantClash {
        uuid: Uuid::from_u128(600),
        declaring_participant_id: Uuid::from_u128(440),
        target_participant_id: Uuid::from_u128(401),
        clash_severity: 20,
    };
    clash.save(&db, true).await?;

    let db = set_up_db(true).await?;
    let clash = ParticipantClash {
        uuid: Uuid::from_u128(600),
        declaring_participant_id: Uuid::from_u128(441),
        target_participant_id: Uuid::from_u128(400),
        clash_severity: 20,
    };
    clash.save(&db, true).await?;

    let clashes = ParticipantClash::get_all_in_tournament(&db, Uuid::from_u128(1)).await?;

    assert_eq!(clashes.len(), 0);

    Ok(())
}