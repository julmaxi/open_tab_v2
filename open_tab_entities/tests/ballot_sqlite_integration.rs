use std::{error::Error, collections::HashMap};

use itertools::Itertools;
use open_tab_entities::domain::{ballot::{Ballot, self, BallotTeam, Speech, SpeakerScore, TeamScore}, tournament::Tournament, round::TournamentRound, debate::TournamentDebate};
use sea_orm::{prelude::*, Database, Statement, ActiveValue};
use migration::{MigratorTrait};

use open_tab_entities::domain::TournamentEntity;

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


async fn test_ballot_roundtrip_in_db(db: &DatabaseConnection, ballot: Ballot, as_insert: bool) -> Result<(), Box<dyn Error>> {
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

async fn test_ballot_roundtrip(ballot: Ballot, as_insert: bool) -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    test_ballot_roundtrip_in_db(&db, ballot, as_insert).await?;
    Ok(())
}

#[tokio::test]
async fn test_empty_ballot_roundtrip() -> Result<(), Box<dyn Error>> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        ..Default::default()
    }, true).await?;

    Ok(())
}


#[tokio::test]
async fn test_preserve_adjudicator_order() -> Result<(), Box<dyn Error>> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![403, 401, 405].into_iter().map(|u| Uuid::from_u128(u as u128)).collect(),
        ..Default::default()
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_set_president() -> Result<(), Box<dyn Error>> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        president: Some(Uuid::from_u128(401)),
        ..Default::default()
    }, true).await?;

    Ok(())
}


#[tokio::test]
async fn test_ballot_teams_roundtrip() -> Result<(), Box<dyn Error>> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        government: BallotTeam {
            team: Some(Uuid::from_u128(200)),
            scores: HashMap::from_iter(
                vec![(Uuid::from_u128(402), TeamScore::Aggregate(140)), (Uuid::from_u128(403), TeamScore::Aggregate(143))].into_iter()
            ),
            ..Default::default()
        },
        opposition: BallotTeam {
            team: Some(Uuid::from_u128(201)),
            scores: HashMap::from_iter(vec![(Uuid::from_u128(401), TeamScore::Aggregate(140))].into_iter())
        },
        ..Default::default()
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_can_not_set_nonexistant_team() -> Result<(), Box<dyn Error>> {
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
async fn judge_not_in_adjudicators_can_not_score_team() -> Result<(), Box<dyn Error>> {
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        government: BallotTeam {
            team: Some(Uuid::from_u128(200)),
            scores: HashMap::from_iter(
                vec![(Uuid::from_u128(405), TeamScore::Aggregate(54))].into_iter(),
            )
        },
        ..Default::default()
    };

    let db = set_up_db(true).await?;
    assert!(ballot.save(&db, true).await.is_err());
    Ok(())
}


#[tokio::test]
async fn test_speeches_roundtrip() -> Result<(), Box<dyn Error>> {
    test_ballot_roundtrip(Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        speeches: vec![
            Speech { speaker: None, role: ballot::SpeechRole::Government, position: 0, scores: HashMap::from_iter(
                vec![(Uuid::from_u128(401), SpeakerScore::Aggregate(54)), (Uuid::from_u128(402), SpeakerScore::Aggregate(32))].into_iter(),
            )
        }],
        ..Default::default()
    }, true).await?;

    Ok(())
}

#[tokio::test]
async fn test_judge_not_in_adjudicators_can_not_score_speech() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: (401..=404).map(|u| Uuid::from_u128(u as u128)).collect(),
        speeches: vec![
            Speech { speaker: None, role: ballot::SpeechRole::Government, position: 0, scores: HashMap::from_iter(
                vec![(Uuid::from_u128(405), SpeakerScore::Aggregate(54))].into_iter(),
            )
        }],
        ..Default::default()
    };

    assert!(ballot.save(&db, true).await.is_err());

    Ok(())
}


#[tokio::test]
async fn test_change_team() -> Result<(), Box<dyn Error>> {
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
async fn test_swap_team() -> Result<(), Box<dyn Error>> {
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
async fn test_remove_adjudicator() -> Result<(), Box<dyn Error>> {
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
async fn test_reorder_adjudicators() -> Result<(), Box<dyn Error>> {
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
async fn test_reorder_annotators_keeps_scores_with_judges() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![Uuid::from_u128(401), Uuid::from_u128(402)],
        government: BallotTeam {
            team: None,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(401), TeamScore::Aggregate(23))])
        },
        opposition: BallotTeam {
            team: None,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(402), TeamScore::Aggregate(53))])
        },
        speeches: vec![
            Speech { speaker: None, role: ballot::SpeechRole::Government, position: 0, scores: HashMap::from_iter(vec![(Uuid::from_u128(401), SpeakerScore::Aggregate(43))]) }
        ],
        ..Default::default()
    };

    ballot.save(&db, true).await?;

    ballot.adjudicators = vec![Uuid::from_u128(402), Uuid::from_u128(401)];

    test_ballot_roundtrip_in_db(&db, ballot, false).await?;

    Ok(())
}


#[tokio::test]
async fn test_remove_adjudicators_deletes_team_scores() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let mut ballot = Ballot {
        uuid: Uuid::from_u128(100),
        adjudicators: vec![Uuid::from_u128(401), Uuid::from_u128(402)],
        government: BallotTeam {
            team: None,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(401), TeamScore::Aggregate(23)), (Uuid::from_u128(402), TeamScore::Aggregate(53))])
        },
        speeches: vec![
            Speech {
                speaker: None,
                role: ballot::SpeechRole::Government,
                position: 0,
                scores: HashMap::from_iter(
                    vec![(Uuid::from_u128(401), SpeakerScore::Aggregate(43))]
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
async fn test_remove_adjudicators_deletes_speech_scores() -> Result<(), Box<dyn Error>> {
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
                    vec![(Uuid::from_u128(402), SpeakerScore::Aggregate(43)), (Uuid::from_u128(401), SpeakerScore::Aggregate(43))]
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
async fn test_get_tournament_from_independent_ballot() -> Result<(), Box<dyn Error>> {
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
async fn test_get_tournament_from_debate_ballot() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;
    let tournament = Tournament {
        uuid: Uuid::from_u128(10),
    };
    tournament.save(&db, true).await?;

    let round = TournamentRound {
        uuid: Uuid::from_u128(21),
        tournament_id: tournament.uuid,
        index: 0
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
        current_ballot_uuid: ballot.uuid
    };
    debate.save(&db, true).await?;

    assert_eq!(ballot.get_tournament(&db).await?, Some(Uuid::from_u128(10)));

    Ok(())
}

#[tokio::test]
async fn test_get_many_preserves_order() -> Result<(), Box<dyn Error>> {
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