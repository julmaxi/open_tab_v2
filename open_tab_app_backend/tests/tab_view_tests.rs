


use migration::MigratorTrait;
use open_tab_entities::{prelude::*, Entity, mock::{make_mock_tournament_with_options, MockOption}, domain::entity::LoadEntity};
use sea_orm::{prelude::*, Database, Statement};

use open_tab_app_backend::views::tab_view::LoadedTabView;

const TAB_TOLERANCE : f64 = 0.0001;

pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, anyhow::Error> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        let mut entities = make_mock_tournament_with_options(MockOption { deterministic_uuids: true, num_teams: 6, num_adjudicators: 5, draw_debates: false, ..Default::default() });

        let ballot_1 = Ballot {
            uuid: Uuid::from_u128(400),
            adjudicators: vec![Uuid::from_u128(3000), Uuid::from_u128(3001), Uuid::from_u128(3002)],
            government: BallotTeam {
                team: Some(Uuid::from_u128(1000)),
                scores: vec![
                    (Uuid::from_u128(3000), TeamScore::Aggregate { total: 120 }),
                    (Uuid::from_u128(3001), TeamScore::Aggregate { total: 20 }),
                    (Uuid::from_u128(3002), TeamScore::Aggregate { total: 100 }),
                ].into_iter().collect(),
                ..Default::default()
            },
            opposition: BallotTeam {
                team: Some(Uuid::from_u128(1001)),
                scores: vec![
                    (Uuid::from_u128(3000), TeamScore::Aggregate { total: 100 }),
                    (Uuid::from_u128(3001), TeamScore::Aggregate { total: 100 }),
                    (Uuid::from_u128(3002), TeamScore::Aggregate { total: 100 }),
                ].into_iter().collect(),
                ..Default::default()
            },
            speeches: vec![
                Speech {
                    speaker: Some(Uuid::from_u128(2000)),
                    role: SpeechRole::Government,
                    is_opt_out: false,
                    position: 0,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 53 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 60 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 70 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2010)),
                    role: SpeechRole::Opposition,
                    is_opt_out: false,
                    position: 0,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2001)),
                    role: SpeechRole::Government,
                    position: 1,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 20 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 21 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 20 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2011)),
                    role: SpeechRole::Opposition,
                    position: 1,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2050)),
                    role: SpeechRole::NonAligned,
                    position: 0,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 80 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 70 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 70 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2050)),
                    role: SpeechRole::NonAligned,
                    position: 1,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 80 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 70 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 71 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2051)),
                    role: SpeechRole::NonAligned,
                    position: 2,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 51 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2012)),
                    role: SpeechRole::Opposition,
                    position: 2,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2002)),
                    role: SpeechRole::Government,
                    position: 2,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3002), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
            ],
            ..Default::default()
        };

        let ballot_2 = Ballot {
            uuid: Uuid::from_u128(410),
            adjudicators: vec![Uuid::from_u128(3000), Uuid::from_u128(3001)],
            government: BallotTeam {
                team: Some(Uuid::from_u128(1003)),
                scores: vec![
                    (Uuid::from_u128(3000), TeamScore::Aggregate { total: 100 }),
                    (Uuid::from_u128(3001), TeamScore::Aggregate { total: 100 }),
                ].into_iter().collect(),
                ..Default::default()
            },
            opposition: BallotTeam {
                team: Some(Uuid::from_u128(1000)),
                scores: vec![
                    (Uuid::from_u128(3000), TeamScore::Aggregate { total: 120 }),
                    (Uuid::from_u128(3001), TeamScore::Aggregate { total: 121 }),
                ].into_iter().collect(),
                ..Default::default()
            },
            speeches: vec![
                Speech {
                    speaker: Some(Uuid::from_u128(2030)),
                    role: SpeechRole::Government,
                    position: 0,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2000)),
                    role: SpeechRole::Opposition,
                    position: 0,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2031)),
                    role: SpeechRole::Government,
                    position: 1,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2001)),
                    role: SpeechRole::Opposition,
                    position: 1,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2050)),
                    role: SpeechRole::NonAligned,
                    position: 0,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2051)),
                    role: SpeechRole::NonAligned,
                    position: 1,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2052)),
                    role: SpeechRole::NonAligned,
                    position: 2,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2002)),
                    role: SpeechRole::Opposition,
                    position: 2,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
                Speech {
                    speaker: Some(Uuid::from_u128(2032)),
                    role: SpeechRole::Government,
                    position: 2,
                    is_opt_out: false,
                    scores: vec![
                        (Uuid::from_u128(3000), SpeakerScore::Aggregate { total: 50 }),
                        (Uuid::from_u128(3001), SpeakerScore::Aggregate { total: 50 }),
                    ].into_iter().collect()
                },
            ],
            ..Default::default()
        };


        let ballots = vec![ballot_1, ballot_2];
    
        let debates = vec![
            TournamentDebate {
                uuid: Uuid::from_u128(200),
                round_id: Uuid::from_u128(100),
                ballot_id: ballots[0].uuid,
                index: 0,
                ..Default::default()
            },
            TournamentDebate {
                uuid: Uuid::from_u128(210),
                round_id: Uuid::from_u128(101),
                ballot_id: ballots[1].uuid,
                index: 0,
                ..Default::default()
            }
        ];

        debates.into_iter().for_each(|d| entities.add(Entity::TournamentDebate(d)));
        ballots.into_iter().for_each(|b| entities.add(Entity::Ballot(b)));
        entities.save_all(&db).await?;
    }
    Ok(db)
}


#[tokio::test]
async fn test_team_ranking_has_all_teams() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    assert_eq!(view.team_tab.len(), 6);

    Ok(())
}

#[tokio::test]
async fn test_team_ranking_has_all_speakers() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    assert_eq!(view.speaker_tab.len(), 18);

    Ok(())
}

#[tokio::test]
async fn test_total_team_score_is_correct_for_faction_team() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    let target_team_entry = view.team_tab.iter().find(|e| e.team_uuid == Uuid::from_u128(1000)).expect("Expected to find team");

    assert!((target_team_entry.total_score - 481.8333333333333).abs() < TAB_TOLERANCE, "Incorrect score: {}", target_team_entry.total_score);
    Ok(())
}

#[tokio::test]
async fn test_total_team_score_is_correct_for_non_aligned_team() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    let target_team_entry = view.team_tab.iter().find(|e| e.team_uuid == Uuid::from_u128(1005)).expect("Expected to find team");

    assert!((target_team_entry.total_score - 347.333333333).abs() < TAB_TOLERANCE, "Incorrect score: {}", target_team_entry.total_score);
    Ok(())
}


#[tokio::test]
async fn test_opt_out_replacement_speaker_receives_only_one_score() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let mut ballot = Ballot::get(&db, Uuid::from_u128(400)).await?;
    ballot.speeches[2].is_opt_out = true;
    ballot.speeches[2].speaker = Some(Uuid::from_u128(2000));
    ballot.save(&db, false).await.unwrap();

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    let replacement_speaker = view.speaker_tab.iter().find(|e| e.speaker_uuid == Uuid::from_u128(2000)).expect("Expected to find speaker");
    assert_eq!(replacement_speaker.avg_score, Some(55.5));

    Ok(())
}


#[tokio::test]
async fn test_double_opt_out_replacement_speaker_receives_only_one_score() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let mut ballot = Ballot::get(&db, Uuid::from_u128(400)).await?;
    ballot.speeches[2].is_opt_out = true;
    ballot.speeches[2].speaker = Some(Uuid::from_u128(2000));
    ballot.speeches[8].is_opt_out = true;
    ballot.speeches[8].speaker = Some(Uuid::from_u128(2000));
    ballot.save(&db, false).await.unwrap();

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    let replacement_speaker = view.speaker_tab.iter().find(|e| e.speaker_uuid == Uuid::from_u128(2000)).expect("Expected to find speaker");
    dbg!(replacement_speaker);
    assert_eq!(replacement_speaker.avg_score, Some(55.5));

    Ok(())
}


#[tokio::test]
async fn test_opt_out_preserves_team_score() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let mut ballot = Ballot::get(&db, Uuid::from_u128(400)).await?;
    ballot.speeches[2].is_opt_out = true;
    ballot.speeches[2].speaker = Some(Uuid::from_u128(2000));
    ballot.save(&db, false).await.unwrap();

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    let target_team_entry = view.team_tab.iter().find(|e| e.team_uuid == Uuid::from_u128(1000)).expect("Expected to find team");
    assert!((target_team_entry.total_score - 481.8333333333333).abs() < TAB_TOLERANCE, "Incorrect score: {}", target_team_entry.total_score);

    Ok(())
}


#[tokio::test]
async fn test_opt_out_adds_smallest_score_for_non_aligned() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;

    let mut ballot = Ballot::get(&db, Uuid::from_u128(400)).await?;
    ballot.speeches[4].is_opt_out = true;
    ballot.save(&db, false).await.unwrap();

    let loaded_view = LoadedTabView::load(&db, Uuid::from_u128(1)).await?;

    let view = loaded_view.view;

    let target_team_entry = view.team_tab.iter().find(|e| e.team_uuid == Uuid::from_u128(1005)).expect("Expected to find team");

    assert!((target_team_entry.total_score - 324.33333).abs() < TAB_TOLERANCE, "Incorrect score: {}", target_team_entry.total_score);

    Ok(())
}
