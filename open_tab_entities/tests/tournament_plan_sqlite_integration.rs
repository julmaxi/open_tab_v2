use std::{error::Error};


use open_tab_entities::{prelude::*, domain::{tournament_break::{BreakType, TournamentBreakSourceRoundType}, entity::LoadEntity, tournament_plan_node::{TournamentPlanNode, PlanNodeType, BreakConfig, RoundGroupConfig}}, mock::{self, MockOption}};
use sea_orm::{prelude::*, Database, Statement};
use migration::{MigratorTrait};

use open_tab_entities::domain::tournament_break::TournamentBreak;
use open_tab_entities::domain::TournamentEntity;

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
        ).save_all_and_log_for_tournament(&db, Uuid::from_u128(1)).await?;
    }
    Ok(db)
}

async fn test_break_roundtrip_in_db(db: &DatabaseConnection, tournament_node: TournamentPlanNode, as_insert: bool) -> Result<(), anyhow::Error> {
    tournament_node.save(db, as_insert).await?;

    let mut saved_break = TournamentPlanNode::get_many(
        db,
        vec![tournament_node.uuid]
    ).await?;

    assert_eq!(saved_break.len(), 1);
    let saved_node = saved_break.pop().unwrap();
    assert_eq!(tournament_node, saved_node);

    Ok(())
}

async fn test_node_roundtrip(tournament_break: TournamentPlanNode, as_insert: bool) -> Result<(), anyhow::Error> {
    let db = set_up_db(true).await?;
    test_break_roundtrip_in_db(&db, tournament_break, as_insert).await?;
    Ok(())
}

#[tokio::test]
async fn test_save_empty_break_node() {
    test_node_roundtrip(
        TournamentPlanNode {
            uuid: Uuid::from_u128(600),
            tournament_id: Uuid::from_u128(1),
            config: PlanNodeType::Break{
                config: BreakConfig::Manual,
                break_id: None
            }
        },
        true
    ).await.unwrap();
}


#[tokio::test]
async fn test_save_empty_round_node() {
    let e = test_node_roundtrip(
        TournamentPlanNode {
            uuid: Uuid::from_u128(600),
            tournament_id: Uuid::from_u128(1),
            config: PlanNodeType::Round { config: RoundGroupConfig::Preliminaries { num_roundtrips: 1 }, rounds: vec![] }
        }
        ,
        true
    ).await;

    if let Err(e) = e {
        panic!("Error: {}", e)
    }
}




#[tokio::test]
async fn test_save_round_node_with_rounds() {
    test_node_roundtrip(
        TournamentPlanNode {
            uuid: Uuid::from_u128(600),
            tournament_id: Uuid::from_u128(1),
            config: PlanNodeType::Round { config: RoundGroupConfig::Preliminaries { num_roundtrips: 1 }, rounds: vec![Uuid::from_u128(100)] }}
        ,
        true
    ).await.unwrap();
}
