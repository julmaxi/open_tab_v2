mod common;
use std::collections::HashMap;

use open_tab_entities::{prelude::Participant, Entity, EntityType, EntityState, EntityGroup, EntityGroupTrait};
use open_tab_server::{sync::{FatLog, SyncRequest, LogEntry, EntityEntry}, participants::ParticipantInfoResponse};
use sea_orm::{prelude::Uuid, DatabaseConnection, IntoActiveModel, ActiveModelTrait};
use tracing_test::traced_test;

use crate::common::FixtureOptions;


#[tokio::test]
#[traced_test]
async fn test_add_participant() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;
    let default_tournament_uuid = Uuid::from_u128(1);

    let participant_uuid = Uuid::from_u128(100_000);
    let participant = Participant {
        uuid: participant_uuid,
        name: "Peter G.".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(
            open_tab_entities::domain::participant::Adjudicator {
                chair_skill: 1,
                panel_skill: 2,
                unavailable_rounds: vec![],
            }
        ),
        tournament_id: default_tournament_uuid,
        institutions: vec![],
        registration_key: None,
        is_anonymous: false
    };
    
    let fake_version = Uuid::from_u128(200_000);

    let mut response = fixture.get(&format!("/api/tournament/{}/log", default_tournament_uuid)).await;
    assert_eq!(response.status(), 200);
    let version = response.json::<FatLog<Entity, EntityType>>().await;
    let last_log = version.log.last().unwrap().uuid;

    let log = FatLog { log: vec![
        LogEntry {
            uuid: fake_version,
            target_type: EntityType::Participant,
            target_uuid: participant.uuid,
            timestamp: chrono::offset::Local::now().naive_utc(),
        }
    ], entities: HashMap::from_iter(
        vec![
            (
                (EntityType::Participant, vec![EntityEntry {
                    uuid: participant_uuid,
                    old_versions: vec![],
                    current_version: fake_version,
                    current_value: EntityState::Exists(Entity::Participant(participant))
                }])
            )
        ].into_iter()
    ) };

    let sync_request = SyncRequest {
        log,
        last_common_ancestor: Some(last_log)
    };

    let response = fixture.post_json(&format!("/api/tournament/{}/log", default_tournament_uuid), sync_request).await;
    
    assert_eq!(response.status(), 200);

    let mut response = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    assert_eq!(response.status(), 200);

    let body = response.json::<ParticipantInfoResponse>().await;
    assert_eq!(body.name, "Peter G.")
}



async fn create_second_tournament(db: DatabaseConnection) {
    let tournament_2_uuid = Uuid::from_u128(2);
    let mut changes = EntityGroup::new();
    let tournament: open_tab_entities::prelude::Tournament = open_tab_entities::domain::tournament::Tournament {
        uuid: tournament_2_uuid,
        annoucements_password: Some("password".into()),
        name: "2".into(),
        ..Default::default()
    };
    changes.add(open_tab_entities::Entity::Tournament(tournament));
    changes.save_all(&db).await.unwrap();    

    let user_tournament = open_tab_entities::schema::user_tournament::Model {
        user_id: Uuid::from_u128(900_000),
        tournament_id: Uuid::from_u128(2)
    };
    user_tournament.into_active_model().insert(&db).await.unwrap();

}


#[tokio::test]
#[traced_test]
async fn test_can_add_participant_to_empty_tournament() {
    let tournament_2_uuid = Uuid::from_u128(2);
    let mut fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        },
        Box::new(create_second_tournament)
    ).await;
    let participant_uuid = Uuid::from_u128(100_000);
    let participant = Participant {
        uuid: participant_uuid,
        name: "Peter G.".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(
            open_tab_entities::domain::participant::Adjudicator {
                chair_skill: 1,
                panel_skill: 2,
                unavailable_rounds: vec![],
            }
        ),
        tournament_id: tournament_2_uuid,
        institutions: vec![],
        registration_key: None,
        is_anonymous: false
    };
    
    let fake_version = Uuid::from_u128(200_000);
    let log = FatLog { log: vec![
        LogEntry {
            uuid: fake_version,
            target_type: EntityType::Participant,
            target_uuid: participant.uuid,
            timestamp: chrono::offset::Local::now().naive_utc(),
        }
    ], entities: HashMap::from_iter(
        vec![
            (
                (EntityType::Participant, vec![EntityEntry {
                    uuid: participant_uuid,
                    old_versions: vec![],
                    current_version: fake_version,
                    current_value: EntityState::Exists(Entity::Participant(participant))
                }])
            )
        ].into_iter()
    ) };

    let sync_request = SyncRequest {
        log,
        last_common_ancestor: None
    };

    let response = fixture.post_json(&format!("/api/tournament/{}/log", tournament_2_uuid), sync_request).await;
    
    assert_eq!(response.status(), 200);

    let mut response = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    assert_eq!(response.status(), 200);

    let body = response.json::<ParticipantInfoResponse>().await;
    assert_eq!(body.name, "Peter G.")
}


#[tokio::test]
#[traced_test]
async fn test_can_not_add_participant_to_other_tournament() {
    let tournament_2_uuid = Uuid::from_u128(2);
    let mut fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        },
        Box::new(create_second_tournament)
    ).await;
    let participant_uuid = Uuid::from_u128(100_000);
    let participant = Participant {
        uuid: participant_uuid,
        name: "Peter G.".into(),
        role: open_tab_entities::domain::participant::ParticipantRole::Adjudicator(
            open_tab_entities::domain::participant::Adjudicator {
                chair_skill: 1,
                panel_skill: 2,
                unavailable_rounds: vec![],
            }
        ),
        tournament_id: Uuid::from_u128(1),
        institutions: vec![],
        registration_key: None,
        is_anonymous: false
    };
    
    let fake_version = Uuid::from_u128(200_000);
    let log = FatLog { log: vec![
        LogEntry {
            uuid: fake_version,
            target_type: EntityType::Participant,
            target_uuid: participant.uuid,
            timestamp: chrono::offset::Local::now().naive_utc(),
        }
    ], entities: HashMap::from_iter(
        vec![
            (
                (EntityType::Participant, vec![EntityEntry {
                    uuid: participant_uuid,
                    old_versions: vec![],
                    current_version: fake_version,
                    current_value: EntityState::Exists(Entity::Participant(participant))
                }])
            )
        ].into_iter()
    ) };

    let sync_request = SyncRequest {
        log,
        last_common_ancestor: None
    };

    let response = fixture.post_json(&format!("/api/tournament/{}/log", tournament_2_uuid), sync_request).await;
    
    assert_eq!(response.status(), 400);

    let response = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    // If the participant does not exist, we are forbidden
    // from accessing it.
    assert_eq!(response.status(), 403);
}


#[tokio::test]
#[traced_test]
async fn test_can_delete_participant() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;
    let default_tournament_uuid = Uuid::from_u128(1);

    let participant_uuid = Uuid::from_u128(2000);
    
    let fake_version = Uuid::from_u128(200_000);
    let response: common::APIResponse = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    assert_eq!(response.status(), 200);

    let mut response = fixture.get(&format!("/api/tournament/{}/log", default_tournament_uuid)).await;
    assert_eq!(response.status(), 200);
    let version = response.json::<FatLog<Entity, EntityType>>().await;
    let last_log = version.log.last().unwrap().uuid;

    let log = FatLog { log: vec![
        LogEntry {
            uuid: fake_version,
            target_type: EntityType::Participant,
            target_uuid: participant_uuid,
            timestamp: chrono::offset::Local::now().naive_utc(),
        }
    ], entities: HashMap::from_iter(
        vec![
            (
                (EntityType::Participant, vec![EntityEntry {
                    uuid: participant_uuid,
                    old_versions: vec![],
                    current_version: fake_version,
                    current_value: EntityState::<Entity, _>::Deleted { uuid: participant_uuid, type_: EntityType::Participant }
                }])
            )
        ].into_iter()
    ) };

    let sync_request = SyncRequest {
        log,
        last_common_ancestor: Some(last_log)
    };

    let response = fixture.post_json(&format!("/api/tournament/{}/log", default_tournament_uuid), sync_request).await;
    
    assert_eq!(response.status(), 200);

    let response: common::APIResponse = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    // If we delete a participant, we can no longer access it
    // with admin rights, since the auth check requires the
    // participant to exist.
    assert_eq!(response.status(), 403);
}

#[tokio::test]
#[traced_test]
async fn test_can_delete_non_existant_participant() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;
    let default_tournament_uuid = Uuid::from_u128(1);

    let participant_uuid = Uuid::from_u128(20_000);
    
    let fake_version = Uuid::from_u128(200_000);
    let response: common::APIResponse = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    assert_eq!(response.status(), 403);

    let mut response = fixture.get(&format!("/api/tournament/{}/log", default_tournament_uuid)).await;
    assert_eq!(response.status(), 200);
    let version = response.json::<FatLog<Entity, EntityType>>().await;
    let last_log = version.log.last().unwrap().uuid;

    let log = FatLog { log: vec![
        LogEntry {
            uuid: fake_version,
            target_type: EntityType::Participant,
            target_uuid: participant_uuid,
            timestamp: chrono::offset::Local::now().naive_utc(),
        }
    ], entities: HashMap::from_iter(
        vec![
            (
                (EntityType::Participant, vec![EntityEntry {
                    uuid: participant_uuid,
                    old_versions: vec![],
                    current_version: fake_version,
                    current_value: EntityState::<Entity, _>::Deleted { uuid: participant_uuid, type_: EntityType::Participant }
                }])
            )
        ].into_iter()
    ) };

    let sync_request = SyncRequest {
        log,
        last_common_ancestor: Some(last_log)
    };

    let response = fixture.post_json(&format!("/api/tournament/{}/log", default_tournament_uuid), sync_request).await;
    
    assert_eq!(response.status(), 200);

    let response: common::APIResponse = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    assert_eq!(response.status(), 403);
}



#[tokio::test]
#[traced_test]
async fn test_can_not_delete_participant_to_other_tournament() {
    let tournament_2_uuid = Uuid::from_u128(2);
    let mut fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        },
        Box::new(create_second_tournament)
    ).await;
    let participant_uuid = Uuid::from_u128(2000);
    
    let fake_version = Uuid::from_u128(200_000);
    let log = FatLog { log: vec![
        LogEntry {
            uuid: fake_version,
            target_type: EntityType::Participant,
            target_uuid: participant_uuid,
            timestamp: chrono::offset::Local::now().naive_utc(),
        }
    ], entities: HashMap::from_iter(
        vec![
            (
                (EntityType::Participant, vec![EntityEntry {
                    uuid: participant_uuid,
                    old_versions: vec![],
                    current_version: fake_version,
                    current_value: EntityState::<Entity, _>::Deleted { uuid: participant_uuid, type_: EntityType::Participant }
                }])
            )
        ].into_iter()
    ) };

    let sync_request = SyncRequest {
        log,
        last_common_ancestor: None
    };

    let response = fixture.post_json(&format!("/api/tournament/{}/log", tournament_2_uuid), sync_request).await;
    
    assert_eq!(response.status(), 400);

    let response = fixture
    .get(&format!("/api/participant/{}", participant_uuid))
    .await;

    assert_eq!(response.status(), 200);
}