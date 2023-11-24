

use migration::MigratorTrait;
use open_tab_entities::{mock, EntityGroupTrait};
use sea_orm::{prelude::*, Database, Statement};

pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, anyhow::Error> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        mock::make_mock_tournament_with_options(mock::MockOption { deterministic_uuids: true, ..Default::default() }).save_all_and_log_for_tournament(&db, Uuid::from_u128(1)).await?;
    }
    Ok(db)
}
