mod common;
use std::{collections::HashMap, default};

use base64::Engine;
use open_tab_entities::{EntityGroup, EntityGroupTrait, domain::{debate_backup_ballot::DebateBackupBallot, entity::LoadEntity, round::{self, RoundState}}, Entity, prelude::{Ballot, BallotTeam, TeamScore, Speech, SpeakerScore}};
use open_tab_server::{tournament::{CreateTournamentRequest, CreateTournamentResponse}, auth::{GetTokenRequest, CreateUserRequest, CreateUserResponse, create_key}, ballot::{GetDebateResponse, GetBallotSubmissionResponse, SubmitBallotRequest, SubmitBallotResponse}, participants::{ParticipantInfoResponse, Motion}};
use sea_orm::{prelude::Uuid, DatabaseConnection, IntoActiveModel, ActiveModelTrait, ActiveValue};
use tracing_test::traced_test;

use crate::common::{FixtureOptions, Auth};

use assert_matches::assert_matches;

#[tokio::test]
#[traced_test]
async fn test_get_adjudicator_info_without_release_does_not_show_draw() {
    let mut fixture: common::Fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;
    
    let mut response = fixture
        .get(&format!("/api/participant/{}", Uuid::from_u128(3000)))
        .await;

    assert_eq!(response.status(), 200);
    let body = response.json::<ParticipantInfoResponse>().await;
    assert_eq!(body.rounds.len(), 3);

    assert_matches!(body.rounds[0].participant_role, None);
    assert_matches!(body.rounds[1].participant_role, None);
    assert_matches!(body.rounds[2].participant_role, None);
}

#[tokio::test]
#[traced_test]
async fn test_get_adjudicator_info_without_release_does_not_show_motion() {
    let mut fixture: common::Fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
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
    EntityGroup::from(vec![
        Entity::TournamentRound(round_1)
    ]).save_all_and_log_for_tournament(&db, Uuid::from_u128(1)).await.unwrap();
}

#[tokio::test]
#[traced_test]
async fn test_get_adjudicator_info_with_future_release_does_not_show_motion() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
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
    round_1.team_motion_release_time = Some(chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1));
    EntityGroup::from(vec![
        Entity::TournamentRound(round_1)
    ]).save_all_and_log_for_tournament(&db, Uuid::from_u128(1)).await.unwrap();
}

#[tokio::test]
#[traced_test]
async fn test_get_adjudicator_info_with_past_release_does_show_motion() {
    let mut fixture: common::Fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
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