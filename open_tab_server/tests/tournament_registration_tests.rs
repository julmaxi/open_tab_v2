mod common;
use open_tab_entities::prelude::Participant;
use open_tab_server::{tournament::{CreateTournamentRequest, CreateTournamentResponse}, auth::{GetTokenRequest, CreateUserRequest, CreateUserResponse, RegisterUserResponse, RegisterParticipantRequest}};
use sea_orm::prelude::Uuid;
use tracing_test::traced_test;

use crate::common::FixtureOptions;


#[tokio::test]
#[traced_test]
async fn test_create_user() {
    let response = common::Fixture::default()
        .await
        .post_json("/api/users", CreateUserRequest {
            password: "testtest".to_string(),
            user_email: None
        })
        .await;

    assert_eq!(response.status(), 200);
}



#[tokio::test]
#[traced_test]
async fn test_can_not_create_token_without_login() {
    let response = common::Fixture::default()
        .await
        .post_json("/api/tokens", GetTokenRequest {
            tournament: None,
        })
        .await;

    assert_eq!(response.status(), 401);
}


#[tokio::test]
#[traced_test]
async fn test_can_create_generic_token_with_login() {
    let mut fixture = common::Fixture::default().await;
    let mut response = fixture
        .post_json("/api/users", CreateUserRequest {
            password: "testtest".to_string(),
            user_email: None
        })
        .await;
    assert_eq!(response.status(), 200);
    let body = response.json::<CreateUserResponse>().await;
    let user_id = body.uuid;

    fixture.auth = common::Auth::Basic {
        username: user_id.to_string(),
        password: "testtest".to_string(),
    };

    let response = fixture
        .post_json("/api/tokens", GetTokenRequest {
            tournament: None,
        })
        .await;
    assert_eq!(response.status(), 200);
}


#[tokio::test]
#[traced_test]
async fn test_can_not_create_generic_token_with_wrong_password() {
    let mut fixture = common::Fixture::default().await;
    let mut response = fixture
        .post_json("/api/users", CreateUserRequest {
            password: "testtest".to_string(),
            user_email: None
        })
        .await;
    assert_eq!(response.status(), 200);
    let body = response.json::<CreateUserResponse>().await;
    let user_id = body.uuid;

    fixture.auth = common::Auth::Basic {
        username: user_id.to_string(),
        password: "wrong".to_string(),
    };

    let response = fixture
        .post_json("/api/tokens", GetTokenRequest {
            tournament: None,
        })
        .await;
    assert_eq!(response.status(), 401);
}


#[tokio::test]
#[traced_test]
async fn test_can_not_create_tournament_without_login() {
    let mut fixture = common::Fixture::default().await;
    let response = fixture
        .post_json("/api/tournaments", CreateTournamentRequest {
            name: "testtest".to_string(),
            uuid: Uuid::from_u128(5)
        })
        .await;
    assert_eq!(response.status(), 401);
}


#[tokio::test]
#[traced_test]
async fn test_can_create_tournament_with_token() {
    let mut fixture = common::Fixture::default().await;
    let (_, token) = fixture.create_user_and_token().await;
    fixture.auth = common::Auth::Bearer {
        token: token,
    };
    let response = fixture
        .post_json("/api/tournaments", CreateTournamentRequest {
            name: "testtest".to_string(),
            uuid: Uuid::from_u128(5)
        })
        .await;
    assert_eq!(response.status(), 200);
}


#[tokio::test]
#[traced_test]
async fn test_tournament_token_can_not_create_generic_token() {
    let mut fixture = common::Fixture::default().await;
    let (_, token) = fixture.create_user_and_token().await;
    fixture.auth = common::Auth::Bearer {
        token: token,
    };
    let mut response = fixture
        .post_json("/api/tournaments", CreateTournamentRequest {
            name: "testtest".to_string(),
            uuid: Uuid::from_u128(5)
        })
        .await;
    assert_eq!(response.status(), 200);
    let body = response.json::<CreateTournamentResponse>().await;
    let tournament_key = body.access_key.unwrap();

    fixture.auth = common::Auth::Bearer {
        token: tournament_key,
    };

    let response = fixture
        .post_json("/api/tokens", GetTokenRequest {
            tournament: None,
        })
        .await;
    assert_eq!(response.status(), 401);
}


#[tokio::test]
#[traced_test]
async fn test_can_register_paricipant_via_key() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;
    let mut registration_secret: [u8; 32] = [0; 32];
    registration_secret[0] = 1;
    registration_secret[1] = 2;

    let encoded_key = Participant::encode_registration_key(Uuid::from_u128(3000), &registration_secret);

    let response = fixture.post_json(&format!("/api/register"), RegisterParticipantRequest {secret: encoded_key}).await;
    assert_eq!(response.status(), 200);
}


#[tokio::test]
#[traced_test]
async fn test_can_not_register_paricipant_via_incorrect_key() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;
    let mut registration_secret: [u8; 32] = [0; 32];
    registration_secret[0] = 1;
    registration_secret[1] = 3;

    let encoded_key = Participant::encode_registration_key(Uuid::from_u128(3000), &registration_secret);

    let response = fixture.post_json(&format!("/api/register"), RegisterParticipantRequest {secret: encoded_key}).await;
    assert_eq!(response.status(), 400);
}


#[tokio::test]
#[traced_test]
async fn test_registering_twice_returns_same_user() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;
    let mut registration_secret: [u8; 32] = [0; 32];
    registration_secret[0] = 1;
    registration_secret[1] = 2;

    let encoded_key = Participant::encode_registration_key(Uuid::from_u128(3000), &registration_secret);

    let mut response = fixture.post_json(&format!("/api/register"), RegisterParticipantRequest {secret: encoded_key.clone() }).await;
    assert_eq!(response.status(), 200);
    let user_id_1 = response.json::<RegisterUserResponse>().await.user_id;

    let mut response = fixture.post_json(&format!("/api/register"), RegisterParticipantRequest {secret: encoded_key }).await;
    assert_eq!(response.status(), 200);
    let user_id_2 = response.json::<RegisterUserResponse>().await.user_id;

    assert_eq!(user_id_1, user_id_2);
}
