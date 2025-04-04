


use open_tab_entities::{prelude::*, domain::{entity::LoadEntity, tournament_plan_node::{TournamentPlanNode, PlanNodeType, BreakConfig, RoundGroupConfig}}, mock::{self, MockOption}};
use sea_orm::{prelude::*, Database, Statement};
use migration::MigratorTrait;


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
        let mut group = mock::make_mock_tournament_with_options(
            MockOption {
                deterministic_uuids: true,
                ..Default::default()
            }
        );

        // Standard mock has all rounds as part of a node.
        // To test node assignment, we add a new round.
        group.add(
            Entity::TournamentRound(
                TournamentRound {
                    uuid: Uuid::from_u128(109),
                    tournament_id: Uuid::from_u128(1),
                    index: 0,
                    ..Default::default()
                }
            )
        );
        
        group.save_all_and_log(&db).await?;

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
                break_id: None,
                eligible_categories: vec![],
                suggested_award_title: None,
                max_breaking_adjudicator_count: None,
                is_only_award: false,
                suggested_break_award_prestige: None
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
            config: PlanNodeType::Round { config: RoundGroupConfig::Preliminaries { num_roundtrips: 1 }, rounds: vec![Uuid::from_u128(109)] }}
        ,
        true
    ).await.unwrap();
}
