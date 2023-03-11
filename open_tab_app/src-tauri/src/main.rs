// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;

use migration::MigratorTrait;
use open_tab_entities::{EntityGroups, domain::tournament::Tournament};
use sea_orm::{prelude::*, Statement, Database};
use tauri::async_runtime::block_on;
use open_tab_entities::prelude::*;
use itertools::Itertools;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


async fn connect_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;
    migration::Migrator::up(&db, None).await.unwrap();
    let _ = db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "PRAGMA foreign_keys = ON;",
        vec![])
    ).await?;

    Ok(db)
}

fn make_mock_tournament() -> EntityGroups {
    let mut groups = EntityGroups::new();

    let tournament_uuid = Uuid::new_v4();
    groups.add(Entity::Tournament(Tournament {
        uuid: tournament_uuid,
        ..Default::default()
    }));
    
    let teams = (0..27).map(|i| {
        let uuid = Uuid::new_v4();
        let name = format!("Team {}", i);
        open_tab_entities::domain::team::Team {
            uuid,
            name,
            tournament_id: tournament_uuid,
        }
    }).collect_vec();

    let speakers = teams.iter().map(|team| {
        let members = (0..2).map(|i| {
            let uuid = Uuid::new_v4();
            let name = format!("Speaker {}", uuid);
            Participant {
                uuid,
                name,
                tournament_id: tournament_uuid,
                role: ParticipantRole::Speaker(Speaker { team_id: Some(team.uuid) }),
            }
        }).collect_vec();

        members
    }).collect_vec();

    let rounds = (0..3).map(|i| {
        let uuid = Uuid::new_v4();
        TournamentRound {
            uuid,
            tournament_id: tournament_uuid,
            index: i,
        }
    }).collect_vec();

    let ballots = rounds.iter().enumerate().map(
        |(round_idx, round)| {
            (0..9).map(
                |i| {
                    let uuid = Uuid::new_v4();
                    Ballot {
                        uuid,
                        adjudicators: (0..3).map(|i| Uuid::new_v4()).collect(),
                        government: BallotTeam {
                            team: Some(teams[i * 3].uuid),
                            ..Default::default()
                        },
                        opposition: BallotTeam {
                            team: Some(teams[i * 3 + 1].uuid),
                            ..Default::default()
                        },
                        speeches: (0..3).map(|speaker_idx| {
                            Speech {
                                speaker:Some(speakers[i*3+2][i].uuid),
                                role: open_tab_entities::domain::ballot::SpeechRole::NonAligned,
                                position: speaker_idx,
                                scores: HashMap::new(),
                            }
                            }).collect(),
                        ..Default::default()
                    }
                }
            )
        }
    ).collect_vec();
}

fn main() {
    let db = block_on(connect_db()).unwrap();
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .manage(db)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
