use std::{error::Error, collections::HashMap, default};

use itertools::Itertools;
use open_tab_entities::domain::{ballot::{Ballot, self, BallotTeam, Speech, SpeakerScore, TeamScore}, tournament::Tournament, round::TournamentRound, debate::TournamentDebate, entity::{LoadEntity, LoadError}};
use sea_orm::{prelude::*, Database, Statement, ActiveValue};
use migration::{MigratorTrait};

use open_tab_entities::domain::TournamentEntity;

pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        let a : open_tab_entities::schema::tournament::ActiveModel = open_tab_entities::schema::tournament::Model {
            uuid: Uuid::from_u128(1),
            annoucements_password: Some("password".into()),
            name: "Test Tournament".into(),
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
                chair_skill: ActiveValue::Set(0),
                panel_skill: ActiveValue::Set(0),
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


async fn test_ballot_roundtrip_in_db(db: &DatabaseConnection, ballot: Ballot, as_insert: bool) -> Result<(), anyhow::Error> {
    ballot.save(db, as_insert).await?;

    let mut saved_ballot = Ballot::get_many(
        db,
        vec![ballot.uuid]
    ).await?;

    assert_eq!(saved_ballot.len(), 1);
    let saved_ballot = saved_ballot.pop().unwrap();
    assert_eq!(ballot, saved_ballot);

    Ok(())
}

async fn test_ballot_roundtrip(ballot: Ballot, as_insert: bool) -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    test_ballot_roundtrip_in_db(&db, ballot, as_insert).await?;
    Ok(())
}

#[tokio::test]
async fn test_empty_ballot_roundtrip() -> Result<(), anyhow::Error> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        ..Default::default()
    }, true).await?;

    Ok(())
}


#[tokio::test]
async fn test_preserve_adjudicator_order() -> Result<(), anyhow::Error> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![403, 401, 405].into_iter().map(|u| Uuid::from_u128(u as u128)).collect(),
        ..Default::default()
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_set_president() -> Result<(), anyhow::Error> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        president: Some(Uuid::from_u128(401)),
        ..Default::default()
    }, true).await?;

    Ok(())
}


#[tokio::test]
async fn test_ballot_teams_roundtrip() -> Result<(), anyhow::Error> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        government: BallotTeam {
            team: Some(Uuid::from_u128(200)),
            scores: HashMap::from_iter(
                vec![(Uuid::from_u128(402), TeamScore::Aggregate { total: 140 }), (Uuid::from_u128(403), TeamScore::Aggregate { total: 143 })].into_iter()
            ),
            ..Default::default()
        },
        opposition: BallotTeam {
            team: Some(Uuid::from_u128(201)),
            scores: HashMap::from_iter(vec![(Uuid::from_u128(401), TeamScore::Aggregate { total: 140 })].into_iter())
        },
        ..Default::default()
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_can_not_set_nonexistant_team() -> Result<(), anyhow::Error> {
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        government: BallotTeam {
            team: Some(Uuid::from_u128(79832)),
            ..Default::default()
        },
        ..Default::default()
    };

    let db = set_up_db(true).await?;
    assert!(ballot.save(&db, true).await.is_err());
    Ok(())
}

#[tokio::test]
async fn judge_not_in_adjudicators_can_not_score_team() -> Result<(), anyhow::Error> {
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        government: BallotTeam {
            team: Some(Uuid::from_u128(200)),
            scores: HashMap::from_iter(
                vec![(Uuid::from_u128(405), TeamScore::Aggregate { total: 54 })].into_iter(),
            )
        },
        ..Default::default()
    };

    let db = set_up_db(true).await?;
    assert!(ballot.save(&db, true).await.is_err());
    Ok(())
}


#[tokio::test]
async fn test_speeches_roundtrip() -> Result<(), anyhow::Error> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        speeches: vec![
            Speech { speaker: None, role: ballot::SpeechRole::Government, position: 0, scores: HashMap::from_iter(
                vec![(Uuid::from_u128(401), SpeakerScore::Aggregate { total: 54 }), (Uuid::from_u128(402), SpeakerScore::Aggregate { total: 32 })].into_iter(),
            )
        }],
        ..Default::default()
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_judge_not_in_adjudicators_can_not_score_speech() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        speeches: vec![
            Speech { speaker: None, role: ballot::SpeechRole::Government, position: 0, scores: HashMap::from_iter(
                vec![(Uuid::from_u128(405), SpeakerScore::Aggregate { total: 54 })].into_iter(),
            )
        }],
        ..Default::default()
    };

    assert!(ballot.save(&db, true).await.is_err());

    Ok(())
}


#[tokio::test]
async fn test_change_team() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        government: BallotTeam {
            team: Some(Uuid::from_u128(200)),
            scores: HashMap::from_iter(vec![].into_iter()),
        },
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.government.team = Some(Uuid::from_u128(201));

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_swap_team() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        government: BallotTeam {
            team: Some(Uuid::from_u128(200)),
            scores: HashMap::from_iter(vec![].into_iter()),
        },
        opposition: BallotTeam {
            team: Some(Uuid::from_u128(201)),
            scores: HashMap::from_iter(vec![].into_iter()),
        },
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.government.team = Some(Uuid::from_u128(201));
    ballot.opposition.team = Some(Uuid::from_u128(200));

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_remove_adjudicator() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![Uuid::from_u128(402), Uuid::from_u128(401)],
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.adjudicators = vec![Uuid::from_u128(401)];

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_reorder_adjudicators() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![Uuid::from_u128(401), Uuid::from_u128(402)],
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.adjudicators = vec![Uuid::from_u128(402), Uuid::from_u128(401)];

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_reorder_annotators_keeps_scores_with_judges() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![Uuid::from_u128(401), Uuid::from_u128(402)],
        government: BallotTeam {
            team: None,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(401), TeamScore::Aggregate { total: 23 })])
        },
        opposition: BallotTeam {
            team: None,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(402), TeamScore::Aggregate { total: 53 })])
        },
        speeches: vec![
            Speech { speaker: None, role: ballot::SpeechRole::Government, position: 0, scores: HashMap::from_iter(vec![(Uuid::from_u128(401), SpeakerScore::Aggregate { total: 43 })]) }
        ],
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.adjudicators = vec![Uuid::from_u128(402), Uuid::from_u128(401)];

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}


#[tokio::test]
async fn test_remove_adjudicators_deletes_team_scores() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![Uuid::from_u128(401), Uuid::from_u128(402)],
        government: BallotTeam {
            team: None,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(401), TeamScore::Aggregate { total: 23 }), (Uuid::from_u128(402), TeamScore::Aggregate { total: 53 })])
        },
        speeches: vec![
            Speech {
                speaker: None,
                role: ballot::SpeechRole::Government,
                position: 0,
                scores: HashMap::from_iter(
                    vec![(Uuid::from_u128(401), SpeakerScore::Aggregate { total: 43 })]
                )
            }
        ],
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.adjudicators = vec![Uuid::from_u128(401)];
    ballot.government.scores.remove(&Uuid::from_u128(402));

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}


#[tokio::test]
async fn test_remove_adjudicators_deletes_speech_scores() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![Uuid::from_u128(401), Uuid::from_u128(402)],
        speeches: vec![
            Speech {
                speaker: None,
                role: ballot::SpeechRole::Government,
                position: 0,
                scores: HashMap::from_iter(
                    vec![(Uuid::from_u128(402), SpeakerScore::Aggregate { total: 43 }), (Uuid::from_u128(401), SpeakerScore::Aggregate { total: 43 })]
                )
            }
        ],
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.adjudicators = vec![Uuid::from_u128(402)];
    ballot.speeches[0].scores.remove(&Uuid::from_u128(401));

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}

#[tokio::test]
async fn test_get_tournament_from_independent_ballot() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![],
        speeches: vec![],
        ..Default::default()
    };
    ballot.save(&db, true).await?;

    assert_eq!(ballot.get_tournament(&db).await?, None);

    Ok(())
}

#[tokio::test]
async fn test_get_tournament_from_debate_ballot() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let tournament = Tournament {
        uuid: Uuid::from_u128(10),
        ..default::Default::default()
    };
    tournament.save(&db, true).await?;

    let round = TournamentRound {
        uuid: Uuid::from_u128(21),
        tournament_id: tournament.uuid,
        index: 0,
        draw_type: None,
        ..Default::default()
    };
    round.save(&db, true).await?;

    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![],
        speeches: vec![],
        ..Default::default()
    };
    ballot.save(&db, true).await?;

    let debate = TournamentDebate {
        uuid: Uuid::from_u128(30),
        round_id: round.uuid,
        index: 0,
        ballot_id: ballot.uuid,
        ..Default::default()
    };
    debate.save(&db, true).await?;

    assert_eq!(ballot.get_tournament(&db).await?, Some(Uuid::from_u128(10)));

    Ok(())
}

#[tokio::test]
async fn test_get_many_preserves_order() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let ballots = (100..=102).map(|i| Ballot {
        uuid: Uuid::from_u128(i),
        adjudicators: vec![],
        speeches: vec![],
        ..Default::default()
    }).collect_vec();
    
    for ballot in ballots {
        ballot.save(&db, true).await?;
    }

    let uuid_order = vec![Uuid::from_u128(101), Uuid::from_u128(100), Uuid::from_u128(102)];
    let retrieved = Ballot::get_many(&db, uuid_order.clone()).await?;

    assert_eq!(retrieved.into_iter().map(|b| b.uuid).collect_vec(), uuid_order);

    Ok(())
}

#[tokio::test]
async fn test_getting_missing_ballot_raises_error() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let ballots = (100..=102).map(|i| Ballot {
        uuid: Uuid::from_u128(i),
        adjudicators: vec![],
        speeches: vec![],
        ..Default::default()
    }).collect_vec();
    
    for ballot in ballots {
        ballot.save(&db, true).await?;
    }

    let uuid_order = vec![Uuid::from_u128(101), Uuid::from_u128(105)];
    let retrieved = Ballot::get_many(&db, uuid_order.clone()).await;

    assert!(retrieved.is_err());

    if let Err(v) = retrieved {
        let err : Result<LoadError, _> = v.downcast();
        let err: LoadError = err.expect("Expected BallotParseError, got something else");

        if let LoadError::EntitiesNotFound {..} = err {
            // pass
        }
        else {
            panic!("Expected BallotParseError::BallotDoesNotExist");
        }
        
    }
    else {
        panic!("Expected BallotParseError::BallotDoesNotExist");
    }

    Ok(())
}

#[tokio::test]
async fn test_try_get_has_correct_order() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let ballots = (100..=102).map(|i| Ballot {
        uuid: Uuid::from_u128(i),
        adjudicators: vec![],
        speeches: vec![],
        ..Default::default()
    }).collect_vec();
    
    for ballot in ballots {
        ballot.save(&db, true).await?;
    }

    let uuid_order = vec![Uuid::from_u128(101), Uuid::from_u128(100), Uuid::from_u128(102)];
    let retrieved = Ballot::try_get_many(&db, uuid_order.clone()).await?;

    assert_eq!(retrieved.into_iter().map(|b| b.unwrap().uuid).collect_vec(), uuid_order);

    Ok(())
}

#[tokio::test]
async fn test_try_get_replaces_missing_with_none() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let ballots = (100..=102).map(|i| Ballot {
        uuid: Uuid::from_u128(i),
        adjudicators: vec![],
        speeches: vec![],
        ..Default::default()
    }).collect_vec();
    
    for ballot in ballots {
        ballot.save(&db, true).await?;
    }

    let uuid_order = vec![Uuid::from_u128(101), Uuid::from_u128(2000), Uuid::from_u128(102)];
    let retrieved = Ballot::try_get_many(&db, uuid_order.clone()).await?;

    assert_eq!(retrieved.into_iter().map(|b| b.map(|b| b.uuid)).collect_vec(), vec![Some(Uuid::from_u128(101)), None, Some(Uuid::from_u128(102))]);

    Ok(())
}


#[tokio::test]
async fn test_delete_ballot_succeeds() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![],
        speeches: vec![],
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    Ballot::delete(&db, Uuid::from_u128(100)).await?;

    let ballot = Ballot::try_get(&db, Uuid::from_u128(100)).await?;
    assert!(ballot.is_none());

    Ok(())
}

#[tokio::test]
async fn test_delete_ballot_deletes_only_targets() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    for i in 100..103 {
        let ballot = Ballot {
            uuid: Uuid::from_u128(i),
            adjudicators: vec![],
            speeches: vec![],
            ..Default::default()
        };
        ballot.save(&db, true).await?;    
    }

    Ballot::delete_many(&db, vec![Uuid::from_u128(100), Uuid::from_u128(102)]).await?;

    for i in 100..103 {
        let ballot = Ballot::try_get(&db, Uuid::from_u128(i)).await?;
        if i == 101 {
            assert!(ballot.is_some());
        }
        else {
            assert!(ballot.is_none());
        }
    }

    Ok(())
}