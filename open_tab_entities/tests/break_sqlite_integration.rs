


use open_tab_entities::{prelude::*, domain::entity::LoadEntity, mock::{self, MockOption}};
use sea_orm::{prelude::*, Database, Statement};
use migration::MigratorTrait;

use open_tab_entities::domain::tournament_break::TournamentBreak;
use open_tab_entities::domain::BoundTournamentEntityTrait;

pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, anyhow::Error> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        mock::make_mock_tournament_with_options(
            MockOption {
                deterministic_uuids: true,
                ..Default::default()
            }
        ).save_all_and_log(&db).await?;
    }
    Ok(db)
}

async fn test_break_roundtrip_in_db(db: &DatabaseConnection, tournament_break: TournamentBreak, as_insert: bool) -> Result<(), anyhow::Error> {
    tournament_break.save(db, as_insert).await?;

    let mut saved_break = TournamentBreak::get_many(
        db,
        vec![tournament_break.uuid]
    ).await?;

    assert_eq!(saved_break.len(), 1);
    let saved_break = saved_break.pop().unwrap();
    assert_eq!(tournament_break, saved_break);

    Ok(())
}

async fn test_break_roundtrip(tournament_break: TournamentBreak, as_insert: bool) -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    test_break_roundtrip_in_db(&db, tournament_break, as_insert).await?;
    Ok(())
}

#[tokio::test]
async fn test_save_empty_break() {
    test_break_roundtrip(
        TournamentBreak {
            uuid: Uuid::from_u128(600),
            tournament_id: Uuid::from_u128(1),
            breaking_teams: vec![],
            breaking_speakers: vec![],
            breaking_adjudicators: vec![]
        },
        true
    ).await.unwrap();
}

#[tokio::test]
async fn test_save_teams() {
    test_break_roundtrip(
        TournamentBreak {
            uuid: Uuid::from_u128(600),
            tournament_id: Uuid::from_u128(1),
            breaking_teams: vec![
                Uuid::from_u128(1004),
                Uuid::from_u128(1002),
                Uuid::from_u128(1000),
                Uuid::from_u128(1001),
            ],
            breaking_speakers: vec![],
            breaking_adjudicators: vec![]
        },
        true
    ).await.unwrap();
}


#[tokio::test]
async fn test_delete_teams() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await.unwrap();
    let mut tournament_break = TournamentBreak {
        uuid: Uuid::from_u128(600),
        tournament_id: Uuid::from_u128(1),
        breaking_teams: vec![
            Uuid::from_u128(1004),
            Uuid::from_u128(1002),
            Uuid::from_u128(1000),
            Uuid::from_u128(1001),
        ],
        breaking_speakers: vec![],
        breaking_adjudicators: vec![]
    };
    tournament_break.save(&db, true).await?;

    tournament_break.breaking_teams = vec![
        Uuid::from_u128(1002),
        Uuid::from_u128(1001),
    ];
    
    test_break_roundtrip(
        tournament_break,
        false
    ).await.unwrap();

    Ok(())
}


#[tokio::test]
async fn test_add_teams() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await.unwrap();
    let mut tournament_break = TournamentBreak {
        uuid: Uuid::from_u128(600),
        tournament_id: Uuid::from_u128(1),
        breaking_teams: vec![
            Uuid::from_u128(1004),
            Uuid::from_u128(1002),
            Uuid::from_u128(1000),
            Uuid::from_u128(1001),
        ],
        breaking_speakers: vec![],
        breaking_adjudicators: vec![]
    };
    tournament_break.save(&db, true).await?;

    tournament_break.breaking_teams = vec![
        Uuid::from_u128(1004),
        Uuid::from_u128(1002),
        Uuid::from_u128(1000),
        Uuid::from_u128(1001),
        Uuid::from_u128(1005),
        Uuid::from_u128(1006),
];
    
    test_break_roundtrip(
        tournament_break,
        false
    ).await.unwrap();

    Ok(())
}


#[tokio::test]
async fn test_save_speakers() {
    test_break_roundtrip(
        TournamentBreak {
            uuid: Uuid::from_u128(600),
            tournament_id: Uuid::from_u128(1),
            breaking_teams: vec![
            ],
            breaking_speakers: vec![
                Uuid::from_u128(2002),
                Uuid::from_u128(2000),
                Uuid::from_u128(2001),
                Uuid::from_u128(2012),
            ],
            breaking_adjudicators: vec![]
        },
        true
    ).await.unwrap();
}


#[tokio::test]
async fn test_save_adjudicators() {
    test_break_roundtrip(
        TournamentBreak {
            uuid: Uuid::from_u128(600),
            tournament_id: Uuid::from_u128(1),
            breaking_teams: vec![],
            breaking_speakers: vec![],
            breaking_adjudicators: vec![
                Uuid::from_u128(3000),
                Uuid::from_u128(3001),
                Uuid::from_u128(3002),
                Uuid::from_u128(3012),
            ]
        },
        true
    ).await.unwrap();
}



#[tokio::test]
async fn test_delete_adjudicators() -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await.unwrap();
    let mut tournament_break = TournamentBreak {
        uuid: Uuid::from_u128(600),
        tournament_id: Uuid::from_u128(1),
        breaking_teams: vec![
        ],
        breaking_speakers: vec![],
        breaking_adjudicators: vec![
            Uuid::from_u128(3002),
            Uuid::from_u128(3000),
            Uuid::from_u128(3001),
            Uuid::from_u128(3012),
        ]
    };
    tournament_break.save(&db, true).await?;

    tournament_break.breaking_adjudicators = vec![
        Uuid::from_u128(3000),
        Uuid::from_u128(3002),
    ];
    
    test_break_roundtrip(
        tournament_break,
        false
    ).await.unwrap();

    Ok(())
}