mod common;



use open_tab_entities::{EntityGroup, domain::entity::LoadEntity, Entity};
use open_tab_server::participants::{ParticipantInfoResponse, Motion};
use sea_orm::{prelude::Uuid, DatabaseConnection};


use crate::common::FixtureOptions;

use assert_matches::assert_matches;

#[tokio::test]

async fn test_get_adjudicator_info_without_release_does_not_show_draw() {
    let mut fixture: common::Fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(3000)),
            ..Default::default()
        }
    ).await;
    
    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;
    assert_eq!(body.rounds.len(), 3);

    assert_matches!(body.rounds[1].participant_role, None);
    assert_matches!(body.rounds[2].participant_role, None);
}

#[tokio::test]

async fn test_get_adjudicator_info_without_release_does_not_show_motion() {
    let mut fixture: common::Fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(3000)),
            ..Default::default()
        }
    ).await;
    
    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;
    assert_eq!(body.rounds.len(), 3);

    assert_matches!(body.rounds[0].motion, Motion::Hidden);
    assert_matches!(body.rounds[1].motion, Motion::Hidden);
    assert_matches!(body.rounds[2].motion, Motion::Hidden);
}

async fn set_future_draw_release(db: DatabaseConnection) {
    let mut round_1 = open_tab_entities::domain::round::TournamentRound::get(&db, Uuid::from_u128(100)).await.unwrap();
    round_1.team_motion_release_time = Some(chrono::Utc::now().naive_utc() + chrono::Duration::minutes(5));
    EntityGroup::new_from_entities(
        Uuid::from_u128(1),
        vec![
        Entity::TournamentRound(round_1)
    ]).save_all_and_log(&db).await.unwrap();
}

#[tokio::test]

async fn test_get_adjudicator_info_with_future_release_does_not_show_motion() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(3000)),
            ..Default::default()
        },
        Box::new(set_future_draw_release)
    ).await;
    
    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;
    assert_eq!(body.rounds.len(), 3);

    assert_matches!(body.rounds[0].motion, Motion::Hidden);
    assert_matches!(body.rounds[1].motion, Motion::Hidden);
    assert_matches!(body.rounds[2].motion, Motion::Hidden);
}

async fn set_past_draw_release(db: DatabaseConnection) {
    let mut round_1 = open_tab_entities::domain::round::TournamentRound::get(&db, Uuid::from_u128(100)).await.unwrap();
    round_1.debate_start_time = Some(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1));
    round_1.team_motion_release_time = Some(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1));
    EntityGroup::new_from_entities(
        Uuid::from_u128(1),
        vec![
        Entity::TournamentRound(round_1)
    ]).save_all_and_log(&db).await.unwrap();
}

#[tokio::test]

async fn test_get_adjudicator_info_with_past_release_does_show_motion() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(3000)),
            ..Default::default()
        },
        Box::new(set_past_draw_release)
    ).await;
    
    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;
    assert_eq!(body.rounds.len(), 3);

    assert_matches!(body.rounds[0].motion, Motion::Shown{..});
    assert_matches!(body.rounds[1].motion, Motion::Hidden);
    assert_matches!(body.rounds[2].motion, Motion::Hidden);
}


#[tokio::test]

async fn test_chair_sees_requests_for_wings_in_current_round() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(3000)),
            ..Default::default()
        },
        Box::new(set_past_draw_release)
    ).await;

    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;

    assert!(body.feedback_submissions.iter().any(
        |submission| {
            submission.target_id == Uuid::from_u128(3001)
            &&
            submission.round_id == Uuid::from_u128(100)
        }
    ));

    assert!(body.feedback_submissions.iter().any(
        |submission| {
            submission.target_id == Uuid::from_u128(3002)
            &&
            submission.round_id == Uuid::from_u128(100)
        }
    ));
}


#[tokio::test]

async fn test_team_member_sees_requests_for_chair_in_current_round() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(2000)),
            ..Default::default()
        },
        Box::new(set_past_draw_release)
    ).await;

    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(2000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;

    assert!(body.feedback_submissions.iter().any(
        |submission| {
            submission.target_id == Uuid::from_u128(3000)
            &&
            submission.round_id == Uuid::from_u128(100)
        }
    ));
}


#[tokio::test]

async fn test_chair_does_not_see_requests_for_future_round() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(3000)),
            ..Default::default()
        },
        Box::new(set_past_draw_release)
    ).await;

    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;

    assert!(!body.feedback_submissions.iter().any(
        |submission| {
            submission.debate_id == Uuid::from_u128(101)
        }
    ));
}

async fn set_past_draw_release_and_make_round_1_silent(db: DatabaseConnection) {
    let mut round_1 = open_tab_entities::domain::round::TournamentRound::get(&db, Uuid::from_u128(100)).await.unwrap();
    round_1.team_motion_release_time = Some(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1));
    round_1.debate_start_time = Some(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1));
    round_1.is_silent = true;
    EntityGroup::new_from_entities(
        Uuid::from_u128(1),
        vec![
        Entity::TournamentRound(round_1)
    ]).save_all_and_log(&db).await.unwrap();

}

#[tokio::test]

async fn test_team_does_not_see_request_for_silent_round() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(2000)),
            ..Default::default()
        },
        Box::new(set_past_draw_release_and_make_round_1_silent)
    ).await;

    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(2000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;

    assert!(!body.feedback_submissions.iter().any(
        |submission| {
            submission.debate_id == Uuid::from_u128(100)
        }
    ));
}

#[tokio::test]

async fn test_wing_does_see_request_for_silent_round() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            use_participant_account: Some(Uuid::from_u128(3000)),
            ..Default::default()
        },
        Box::new(set_past_draw_release_and_make_round_1_silent)
    ).await;

    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);

    let body = response.json::<ParticipantInfoResponse>().await;

    assert!(body.feedback_submissions.iter().any(
        |submission| {
            submission.round_id == Uuid::from_u128(100)
        }
    ));
}
