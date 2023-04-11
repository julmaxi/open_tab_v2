use std::{error::Error, collections::HashMap, default};

use itertools::Itertools;
use migration::MigratorTrait;
use open_tab_entities::{prelude::{Ballot, Tournament, Speech, TeamScore, SpeakerScore}, EntityGroups, Entity, mock::{make_mock_tournament_with_options, MockOption}};
use sea_orm::{prelude::*, Database, Statement, TransactionTrait};


use open_tab_app_backend::{UpdateDrawAction, views::LoadedView, draw_view::{DrawBallot, DrawTeam, DrawAdjudicator, DrawSpeaker, LoadedDrawView}, ActionTrait};


pub async fn set_up_db(with_mock_env: bool) -> Result<DatabaseConnection, Box<dyn Error>> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let r = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    if with_mock_env {
        let entities = make_mock_tournament_with_options(MockOption { deterministic_uuids: true, ..Default::default() });
        entities.save_all(&db).await?;
    }
    Ok(db)
}


#[tokio::test]
async fn test_view_updates_when_ballot_updates() -> Result<(), Box<dyn Error>> {
    let db = set_up_db(true).await?;

    let mut loaded_view = LoadedDrawView::load(&db, Uuid::from_u128(100)).await?;

    let changed_ballot = Ballot {
        uuid: Uuid::from_u128(400),
        ..Default::default()
    };

    let mut changes = EntityGroups::new();
    changes.add(Entity::Ballot(changed_ballot));

    let transaction = db.begin().await?;
    let updates = loaded_view.update_and_get_changes(&transaction, &changes).await?;
    transaction.rollback().await?;

    if let Some(updates) = updates {
        assert!(updates.contains_key("debates.0.ballot"));
    }
    else {
        panic!("Expected updates");
    }

    Ok(())
}
