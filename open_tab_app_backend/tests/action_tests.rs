use std::{error::Error, collections::HashMap, default};

use itertools::Itertools;
use migration::MigratorTrait;
use open_tab_entities::{prelude::{Ballot, Tournament, Speech, TeamScore, SpeakerScore}, EntityGroups, Entity, mock::{make_mock_tournament_with_options, MockOption}};
use sea_orm::{prelude::*, Database, Statement};


use open_tab_app_backend::{actions::UpdateDrawAction, draw_view::{DrawBallot, DrawTeam, DrawAdjudicator, DrawSpeaker}, actions::ActionTrait};


pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, Box<dyn Error>> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        let entities = make_mock_tournament_with_options(MockOption {deterministic_uuids: true, ..Default::default()});
        entities.save_all(&db).await?;
    }
    Ok(db)
}

#[tokio::test]
async fn test_insert_new_empty_ballot() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(false).await?;

    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(200),
            government: None,
            opposition: None,
            non_aligned_speakers: vec![],
            adjudicators: vec![],
            president: None
        }]
    };

    let changes = action.get_changes(&db).await?;

    assert_eq!(changes.ballots.len(), 1);

    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(200)]).await?[0];

    assert_eq!(ballot.government.team, None);
    assert_eq!(ballot.opposition.team, None);

    assert_eq!(ballot.adjudicators.len(), 0);
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.president, None);

    Ok(())
}

#[tokio::test]
async fn test_update_ballot_saves_team() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(421),
            government: Some(DrawTeam {
                uuid: Uuid::from_u128(1002),
                ..Default::default()
            }),
            opposition: Some(DrawTeam {
                uuid: Uuid::from_u128(1001),
                ..Default::default()
            }),
            non_aligned_speakers: vec![],
            adjudicators: vec![],
            president: None
        }]
    };

    let changes = action.get_changes(&db).await?;

    assert_eq!(changes.ballots.len(), 1);

    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?[0];

    assert_eq!(ballot.government.team, Some(Uuid::from_u128(1002)));
    assert_eq!(ballot.opposition.team, Some(Uuid::from_u128(1001)));

    assert_eq!(ballot.adjudicators.len(), 0);
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.president, None);

    Ok(())
}

#[tokio::test]
async fn test_update_ballot_saves_adjudicators() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(421),
            government: None,
            opposition: None,
            non_aligned_speakers: vec![],
            adjudicators: vec![
                DrawAdjudicator { uuid: Uuid::from_u128(3012), ..Default::default() },
                DrawAdjudicator { uuid: Uuid::from_u128(3001), ..Default::default() },
                DrawAdjudicator { uuid: Uuid::from_u128(3002), ..Default::default() },
            ],
            president: Some(DrawAdjudicator { uuid: Uuid::from_u128(3006), ..Default::default()})
        }]
    };

    let changes = action.get_changes(&db).await?;

    assert_eq!(changes.ballots.len(), 1);

    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?[0];

    assert_eq!(ballot.government.team, None);
    assert_eq!(ballot.opposition.team, None);

    assert_eq!(ballot.adjudicators, vec![3012, 3001, 3002].into_iter().map(Uuid::from_u128).collect_vec());
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.president, Some(Uuid::from_u128(3006)));

    Ok(())
}

#[tokio::test]
async fn test_update_ballot_saves_non_aligned() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(421),
            government: None,
            opposition: None,
            non_aligned_speakers: vec![
                DrawSpeaker {
                    uuid: Uuid::from_u128(2002),
                    ..Default::default()
                },
                DrawSpeaker {
                    uuid: Uuid::from_u128(2051),
                    ..Default::default()
                },
                DrawSpeaker {
                    uuid: Uuid::from_u128(2012),
                    ..Default::default()
                },
            ],
            adjudicators: vec![],
            president: None
        }]
    };

    let changes = action.get_changes(&db).await?;

    assert_eq!(changes.ballots.len(), 1);

    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?[0];

    assert_eq!(ballot.government.team, None);
    assert_eq!(ballot.opposition.team, None);

    assert_eq!(ballot.adjudicators.len(), 0);
    assert_eq!(ballot.speeches, vec![2002, 2051, 2012].into_iter().enumerate().map(|(idx, id)| {
        Speech {
            speaker: Some(Uuid::from_u128(id)),
            role: open_tab_entities::prelude::SpeechRole::NonAligned,
            position: idx as u8,
            scores: HashMap::new(),
        }}).collect_vec()
    );
    assert_eq!(ballot.president, None);

    Ok(())
}


#[tokio::test]
async fn test_changing_adjudicator_order_does_not_delete_scores() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let mut prev_ballot = Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?.pop().unwrap();
    prev_ballot.adjudicators = vec![3003, 3001, 3002].into_iter().map(Uuid::from_u128).collect_vec();
    prev_ballot.government.scores.insert(Uuid::from_u128(3003), TeamScore::Aggregate(123));
    
    prev_ballot.speeches = vec![
        Speech {
            speaker: None,
            role: open_tab_entities::prelude::SpeechRole::Government,
            position: 0,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(3003), SpeakerScore::Aggregate(61))].into_iter()),
        }
    ];
    prev_ballot.save(&db, false).await?;
    
    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(421),
            government: None,
            opposition: None,
            non_aligned_speakers: vec![],
            adjudicators: vec![3002, 3001, 3003].into_iter().map(|uuid| DrawAdjudicator { uuid: Uuid::from_u128(uuid), ..Default::default() }).collect_vec(),
            president: None
        }]
    };

    let changes = action.get_changes(&db).await?;
    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?[0];

    assert_eq!(ballot.speeches[0].scores.get(&Uuid::from_u128(3003)), Some(&SpeakerScore::Aggregate(61)));
    assert_eq!(ballot.government.scores.get(&Uuid::from_u128(3003)), Some(&TeamScore::Aggregate(123)));
    assert_eq!(ballot.president, None);

    Ok(())
}


#[tokio::test]
async fn test_delete_adjudicator_with_scores_deletes_both() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let mut prev_ballot = Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?.pop().unwrap();
    prev_ballot.adjudicators = vec![3003, 3001, 3002].into_iter().map(Uuid::from_u128).collect_vec();
    prev_ballot.government.scores.insert(Uuid::from_u128(3003), TeamScore::Aggregate(123));
    
    prev_ballot.speeches = vec![
        Speech {
            speaker: None,
            role: open_tab_entities::prelude::SpeechRole::Government,
            position: 0,
            scores: HashMap::from_iter(vec![(Uuid::from_u128(3003), SpeakerScore::Aggregate(61))].into_iter()),
        }
    ];
    prev_ballot.save(&db, false).await?;
    
    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(421),
            government: None,
            opposition: None,
            non_aligned_speakers: vec![],
            adjudicators: vec![3002, 3001].into_iter().map(|uuid| DrawAdjudicator { uuid: Uuid::from_u128(uuid), ..Default::default() }).collect_vec(),
            president: None
        }]
    };

    let changes = action.get_changes(&db).await?;
    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?[0];
    assert!(!ballot.adjudicators.contains(&Uuid::from_u128(3003)));

    assert_eq!(ballot.speeches[0].scores.len(), 0);
    assert_eq!(ballot.government.scores.len(), 0);
    assert_eq!(ballot.president, None);

    Ok(())
}



#[tokio::test]
async fn test_change_non_aligned_with_additional() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let mut prev_ballot = Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?.pop().unwrap();
    prev_ballot.speeches = vec![
        Speech {
            speaker: Some(Uuid::from_u128(2002)),
            role: open_tab_entities::prelude::SpeechRole::NonAligned,
            position: 0,
            scores: HashMap::new(),
        },
        Speech {
            speaker: Some(Uuid::from_u128(2051)),
            role: open_tab_entities::prelude::SpeechRole::NonAligned,
            position: 1,
            scores: HashMap::new(),
        }
    ];
    
    prev_ballot.save(&db, false).await?;
    
    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(421),
            government: None,
            opposition: None,
            non_aligned_speakers: vec![
                DrawSpeaker { uuid: Uuid::from_u128(2051), ..Default::default() },
                DrawSpeaker { uuid: Uuid::from_u128(2002), ..Default::default() },
                DrawSpeaker { uuid: Uuid::from_u128(2070), ..Default::default() },
            ],
            adjudicators: vec![],
            president: None
        }]
    };

    let changes = action.get_changes(&db).await?;
    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?[0];

    assert_eq!(ballot.speeches.iter().map(|s| s.speaker).collect_vec(), vec![
        Some(Uuid::from_u128(2051)),
        Some(Uuid::from_u128(2002)),
        Some(Uuid::from_u128(2070))
    ]);
    assert_eq!(ballot.government.scores.len(), 0);
    assert_eq!(ballot.president, None);

    Ok(())
}


#[tokio::test]
async fn test_change_non_aligned_with_fewer() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let mut prev_ballot = Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?.pop().unwrap();
    prev_ballot.speeches = vec![
        Speech {
            speaker: Some(Uuid::from_u128(2002)),
            role: open_tab_entities::prelude::SpeechRole::NonAligned,
            position: 0,
            scores: HashMap::new(),
        },
        Speech {
            speaker: Some(Uuid::from_u128(2051)),
            role: open_tab_entities::prelude::SpeechRole::NonAligned,
            position: 1,
            scores: HashMap::new(),
        },
        Speech {
            speaker: Some(Uuid::from_u128(2070)),
            role: open_tab_entities::prelude::SpeechRole::NonAligned,
            position: 2,
            scores: HashMap::new(),
        }
    ];
    
    prev_ballot.save(&db, false).await?;
    
    let action = UpdateDrawAction {
        updated_ballots: vec![DrawBallot {
            uuid: Uuid::from_u128(421),
            government: None,
            opposition: None,
            non_aligned_speakers: vec![
                DrawSpeaker { uuid: Uuid::from_u128(2051), ..Default::default() },
                DrawSpeaker { uuid: Uuid::from_u128(2002), ..Default::default() },
            ],
            adjudicators: vec![],
            president: None
        }]
    };

    let changes = action.get_changes(&db).await?;
    changes.save_all(&db).await?;

    let ballot = &Ballot::get_many(&db, vec![Uuid::from_u128(421)]).await?[0];

    assert_eq!(ballot.speeches.iter().map(|s| s.speaker).collect_vec(), vec![
        Some(Uuid::from_u128(2051)),
        Some(Uuid::from_u128(2002)),
    ]);
    assert_eq!(ballot.government.scores.len(), 0);
    assert_eq!(ballot.president, None);

    Ok(())
}
