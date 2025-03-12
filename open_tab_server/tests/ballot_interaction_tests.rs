mod common;
use std::collections::HashMap;

use base64::Engine;
use open_tab_entities::{EntityGroup, domain::entity::LoadEntity, Entity, prelude::{Ballot, BallotTeam, TeamScore, Speech, SpeakerScore}};
use open_tab_server::{auth::create_key, ballot::{GetDebateResponse, GetBallotSubmissionResponse, SubmitBallotRequest, SubmitBallotResponse}};
use sea_orm::{prelude::Uuid, DatabaseConnection, IntoActiveModel, ActiveModelTrait};
use tracing_test::traced_test;

use crate::common::{FixtureOptions, Auth};


fn make_team_score_map(scores: Vec<(u128, i16)>) -> HashMap<Uuid, TeamScore> {
    HashMap::from_iter(
        scores.into_iter().map(|(uuid, score)| {
            (
                Uuid::from_u128(uuid),
                TeamScore::new_aggregate(score)
            )
        })
    )
}

fn make_speaker_score_map(scores: Vec<(u128, i16)>) -> HashMap<Uuid, SpeakerScore> {
    HashMap::from_iter(
        scores.into_iter().map(
            |(uuid, score)| {
                (
                    Uuid::from_u128(uuid),
                    SpeakerScore::new_aggregate(score)
                )
            }
        )
    )
}

fn make_demo_ballot() -> Ballot {
    Ballot {
        adjudicators: vec![Uuid::from_u128(3000), Uuid::from_u128(3001), Uuid::from_u128(3002)],
        government: BallotTeam {
            team: Some(Uuid::from_u128(1000)),
            scores: make_team_score_map(vec!
                [
                    (3000, 50), (3001, 100), (3002, 75),
                ]
            ),
            ..Default::default()
        },
        opposition: BallotTeam {
            team: Some(Uuid::from_u128(1001)),
            scores: make_team_score_map(vec!
                [
                    (3000, 50), (3002, 75),
                ]
            ),
            ..Default::default()
        },
        speeches: vec![
            Speech {
                speaker: Some(Uuid::from_u128(2000)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 0,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::Government
            },
            Speech {
                speaker: Some(Uuid::from_u128(2010)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 0,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::Opposition
            },
            Speech {
                speaker: Some(Uuid::from_u128(2001)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 1,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::Opposition
            },
            Speech {
                speaker: Some(Uuid::from_u128(2011)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 1,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::Government
            },

            Speech {
                speaker: Some(Uuid::from_u128(2020)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 0,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::NonAligned
            },
            Speech {
                speaker: Some(Uuid::from_u128(2021)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 1,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::NonAligned
            },
            Speech {
                speaker: Some(Uuid::from_u128(2022)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 2,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::NonAligned
            },

            Speech {
                speaker: Some(Uuid::from_u128(2002)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 2,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::Opposition
            },
            Speech {
                speaker: Some(Uuid::from_u128(2012)),
                scores: make_speaker_score_map(vec![(3000, 50), (3001, 100), (3002, 75)]),
                position: 2,
                is_opt_out: false,
                role: open_tab_entities::prelude::SpeechRole::Government
            },

        ],
        ..Default::default()
    }
}


#[tokio::test]
#[traced_test]
async fn test_get_empty_ballot() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;

    let mut response = fixture.get(&format!("/api/debate/{}", Uuid::from_u128(200))).await;
    assert_eq!(response.status(), 200);

    let ballot : GetDebateResponse = response.json().await;

    assert_eq!(
        ballot.ballot.government.uuid,
        Uuid::from_u128(1000)
    );
    assert_eq!(
        ballot.ballot.opposition.uuid,
        Uuid::from_u128(1001)
    );
    assert_eq!(
        ballot.ballot.speeches.len(),
        9
    );
}


#[tokio::test]
#[traced_test]
async fn test_submit_ballot_adds_entry() {
    let mut fixture = common::Fixture::new(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        }
    ).await;

    let mut response = fixture.post_json(
        &format!("/api/debate/{}/submissions", Uuid::from_u128(200)),
        SubmitBallotRequest {
            ballot: make_demo_ballot()
        }
    ).await;
    assert_eq!(response.status(), 200);

    let ballot_id = response.json::<SubmitBallotResponse>().await.submission_id;

    let mut response = fixture.get(&format!("/api/submission/{}", ballot_id)).await;
    assert_eq!(response.status(), 200);

    let ballot : GetBallotSubmissionResponse = response.json().await;

    assert_eq!(
        ballot.ballot.government.uuid,
        Uuid::from_u128(1000)
    );
    assert_eq!(
        ballot.ballot.opposition.uuid,
        Uuid::from_u128(1001)
    );
    assert_eq!(
        ballot.ballot.speeches.len(),
        9
    );
    assert_eq!(
        ballot.ballot.speeches[0].scores.get(&Uuid::from_u128(3000)).unwrap(),
        &50
    );
    assert_eq!(
        ballot.ballot.government.scores.get(&Uuid::from_u128(3000)).unwrap(),
        &50
    );
}

fn get_test_user_key() -> open_tab_entities::schema::user_access_key::Model {
    let raw_key = [0, 0, 0, 1];
    create_key(&raw_key, Uuid::from_u128(13000), None, None, false).unwrap()
}

async fn create_test_user(db: DatabaseConnection) {
    let user: open_tab_entities::schema::user::Model = open_tab_entities::schema::user::Model {
        uuid: Uuid::from_u128(13000),
        password_hash: "".to_string(), // Empty password hash
        user_email: None
    };
    let key = get_test_user_key();
    let user_participant = open_tab_entities::schema::user_participant::Model {
        user_id: Uuid::from_u128(13000),
        participant_id: Uuid::from_u128(3000),
        claim_time: chrono::Utc::now().naive_utc(),
    };

    user.into_active_model().insert(&db).await.unwrap();
    key.into_active_model().insert(&db).await.unwrap();
    user_participant.into_active_model().insert(&db).await.unwrap();    
}

async fn create_test_user_and_open_r1(db: DatabaseConnection) {
    create_test_user(db.clone()).await;

    let mut round_1 = open_tab_entities::domain::round::TournamentRound::get(
        &db,
        Uuid::from_u128(100),
    ).await.unwrap();
    round_1.draw_release_time = Some(chrono::Utc::now().naive_utc());
    EntityGroup::new_from_entities(
        Uuid::from_u128(1),
        vec![Entity::TournamentRound(round_1)]
    ).save_all_and_log(&db).await.unwrap();
}

/*
async fn create_test_user_and_close_r1(db: DatabaseConnection) {
    create_test_user(db.clone()).await;

    let mut round_1 = open_tab_entities::domain::round::TournamentRound::get(
        &db,
        Uuid::from_u128(100),
    ).await.unwrap();
    round_1.state = RoundState::Finished;
    EntityGroup::new_with_entities(vec![Entity::TournamentRound(round_1)]).save_all_and_log_for_tournament(&db, Uuid::from_u128(1)).await.unwrap();
}
 */

#[tokio::test]
#[traced_test]
async fn test_chair_can_submit_ballot() {
    let mut fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        },
        create_test_user_and_open_r1
    ).await;
    let auth = Auth::Bearer { token: base64::engine::general_purpose::URL_SAFE.encode(&[0, 0, 0, 1]) };
    fixture.auth = auth;

    let response = fixture.post_json(
        &format!("/api/debate/{}/submissions", Uuid::from_u128(200)),
        SubmitBallotRequest {
            ballot: make_demo_ballot()
        }
    ).await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
#[traced_test]
async fn test_chair_can_not_submit_ballot_for_inactive_round() {
    let mut fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        },
        create_test_user
    ).await;
    let auth = Auth::Bearer { token: base64::engine::general_purpose::URL_SAFE.encode(&[0, 0, 0, 1]) };
    fixture.auth = auth;

    let response = fixture.post_json(
        &format!("/api/debate/{}/submissions", Uuid::from_u128(210)),
        SubmitBallotRequest {
            ballot: make_demo_ballot()
        }
    ).await;
    assert_eq!(response.status(), 403);
}


/*
#[tokio::test]
#[traced_test]
async fn test_chair_can_not_submit_ballot_for_finished_round() {
    let mut fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        },
        create_test_user_and_close_r1
    ).await;
    let auth = Auth::Bearer { token: base64::engine::general_purpose::URL_SAFE.encode(&[0, 0, 0, 1]) };
    fixture.auth = auth;

    let response = fixture.post_json(
        &format!("/api/debate/{}/submissions", Uuid::from_u128(200)),
        SubmitBallotRequest {
            ballot: make_demo_ballot()
        }
    ).await;
    assert_eq!(response.status(), 403);
}
 */


#[tokio::test]
#[traced_test]
async fn test_chair_in_other_room_cannot_submit_ballot() {
    let mut fixture = common::Fixture::new_with_setup(
        FixtureOptions {
            mock_default_tournament: true,
            ..Default::default()
        },
        create_test_user_and_open_r1
    ).await;
    let auth = Auth::Bearer { token: base64::engine::general_purpose::URL_SAFE.encode(&[0, 0, 0, 1]) };
    fixture.auth = auth;

    let response = fixture.post_json(
        &format!("/api/debate/{}/submissions", Uuid::from_u128(201)),
        SubmitBallotRequest {
            ballot: make_demo_ballot()
        }
    ).await;
    assert_eq!(response.status(), 403);
}