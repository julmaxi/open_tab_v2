// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, error::Error, hash::Hash};

use migration::MigratorTrait;
use open_tab_entities::{EntityGroups, domain::{tournament::Tournament, ballot::SpeechRole}, schema::{adjudicator, self}};
use sea_orm::{prelude::*, Statement, Database};
use tauri::{async_runtime::block_on, State};
use open_tab_entities::prelude::*;
use itertools::{Itertools, izip};
use serde::{Serialize, Deserialize};

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

    let mock_data = make_mock_tournament();
    let tournament_uuid = mock_data.tournaments[0].uuid.clone();
    mock_data.save_all_with_options(&db, true).await.unwrap();
    mock_data.save_log_with_tournament_id(&db, tournament_uuid).await.unwrap();


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
        let members = (0..3).map(|i| {
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

    let adjudicators = (0..27).map(|i| {
        let uuid = Uuid::new_v4();
        let name = format!("Adjudicator {}", uuid);
            Participant {
                uuid,
                name,
                tournament_id: tournament_uuid,
                role: ParticipantRole::Adjudicator(Adjudicator { }),
            }
    }).collect_vec();

    let rounds = (0..=3).map(|i| {
        let uuid = Uuid::from_u128(i as u128);
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
                        adjudicators: (0..3).map(|i| adjudicators[i * 3 + i].uuid).collect(),
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
                                speaker:Some(speakers[i][speaker_idx].uuid),
                                role: open_tab_entities::domain::ballot::SpeechRole::NonAligned,
                                position: speaker_idx as u8,
                                scores: HashMap::new(),
                            }
                            }).collect(),
                        ..Default::default()
                    }
                }
            ).collect_vec()
    }).collect_vec();

    let debates = ballots.iter().enumerate().map(|(round_idx, round_ballots)| {
        let round_debates = round_ballots.iter().enumerate().map(|(debate_idx, ballot)| {
            let uuid = Uuid::new_v4();
            TournamentDebate {
                uuid,
                round_id: rounds[round_idx].uuid,
                current_ballot_uuid: ballot.uuid,
                index: debate_idx as u64
            }
        }).collect_vec();
        round_debates
    }).collect_vec();

    teams.into_iter().for_each(|team| groups.add(Entity::Team(team)));
    speakers.into_iter().flatten().for_each(|speaker| groups.add(Entity::Participant(speaker)));
    adjudicators.into_iter().for_each(|adjudicator| groups.add(Entity::Participant(adjudicator)));
    rounds.into_iter().for_each(|round| groups.add(Entity::TournamentRound(round)));
    ballots.into_iter().flatten().for_each(|ballot| groups.add(Entity::Ballot(ballot)));
    debates.into_iter().flatten().for_each(|debate| groups.add(Entity::TournamentDebate(debate)));

    groups
}



#[derive(Debug, Clone, Serialize, Deserialize)]
struct DrawView {
    round_uuid: Uuid,
    debates: Vec<DrawDebate>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DrawDebate {
    uuid: Uuid,
    index: usize,
    government: Option<DrawTeam>,
    opposition: Option<DrawTeam>,
    non_aligned_speakers: Vec<DrawSpeaker>,
    adjudicators: Vec<DrawAdjudicator>,
    president: Option<DrawAdjudicator>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DrawTeam {
    uuid: Uuid,
    name: String,
    members: Vec<DrawSpeaker>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DrawSpeaker {
    uuid: Uuid,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DrawAdjudicator {
    uuid: Uuid,
    name: String,
}

impl DrawView {
    fn draw_speaker_from_uuid(speaker_uuid: Uuid, speakers_by_id: &HashMap<Uuid, Participant>) -> DrawSpeaker {
        let speaker = speakers_by_id.get(&speaker_uuid).unwrap();
        DrawSpeaker {
            uuid: speaker.uuid,
            name: speaker.name.clone()
        }
    }

    fn draw_team_from_ballot_team(team: &BallotTeam, teams_by_id: &HashMap<Uuid, Team>, speakers_by_id: &HashMap<Uuid, Participant>, team_members: &HashMap<Uuid, Vec<Uuid>>) -> Option<DrawTeam> {
        if let Some(team_uuid) = team.team {
            Some(DrawTeam {
                uuid: team_uuid,
                name: teams_by_id.get(&team_uuid).unwrap().name.clone(),
                members: team_members.get(&team_uuid).unwrap().iter().map(|speaker_uuid| {
                    Self::draw_speaker_from_uuid(*speaker_uuid, speakers_by_id)
                }).collect()
            })
        }
        else {
            None
        }
    }

    fn draw_adjudicator_from_uuid(adjudicator_uuid: Uuid, adjudicators_by_id: &HashMap<Uuid, Participant>) -> DrawAdjudicator {
        let adjudicator = adjudicators_by_id.get(&adjudicator_uuid).unwrap();
        DrawAdjudicator {
            uuid: adjudicator.uuid,
            name: adjudicator.name.clone()
        }
    }

    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<DrawView, Box<dyn Error>> where C: ConnectionTrait {
        let round = schema::tournament_round::Entity::find_by_id(round_uuid).one(db).await?.expect("Round not found");
        let debates = schema::tournament_debate::Entity::find().filter(schema::tournament_debate::Column::RoundId.eq(round_uuid)).all(db).await?;
        let debate_uuids = debates.iter().map(|debate| debate.uuid).collect_vec();

        let ballot_uuids = debates.iter().map(|debate| debate.ballot_id).collect_vec();

        let ballots = Ballot::get_many(db, ballot_uuids).await?;
        let all_participants = Participant::get_all_in_tournament(db, round.tournament_id).await?;
        let team_members = all_participants.iter().filter_map(|speaker| {
            if let ParticipantRole::Speaker(speaker_info) = &speaker.role {
                if let Some(team_uuid) = speaker_info.team_id {
                    Some((team_uuid, speaker.uuid))
                }
                else {
                    None
                }
            }
            else {
                None
            }
        }).into_group_map();
        let teams_by_id = Team::get_all_in_tournament(db, round.tournament_id).await?.into_iter().map(|team| (team.uuid, team)).collect::<HashMap<_, _>>();
        let participants_by_id = all_participants.into_iter().map(|speaker| (speaker.uuid, speaker)).collect::<HashMap<_, _>>();

        let debates = izip![debates, ballots.into_iter()].map(
            |(debate, debate_ballot)| {
                DrawDebate {
                    uuid: debate.uuid,
                    index: debate.index as usize,
                    government: Self::draw_team_from_ballot_team(&debate_ballot.government, &teams_by_id, &participants_by_id, &team_members),
                    opposition: Self::draw_team_from_ballot_team(&debate_ballot.opposition, &teams_by_id, &participants_by_id, &team_members),
                    non_aligned_speakers: debate_ballot.speeches.iter().filter_map(|speech| {
                        if speech.role == SpeechRole::NonAligned {
                            if let Some(speaker_uuid) = speech.speaker {
                                Some(Self::draw_speaker_from_uuid(speaker_uuid, &participants_by_id))
                            }
                            else {
                                None
                            }
                        }
                        else {
                            None
                        }
                    }).collect(),
                    adjudicators: debate_ballot.adjudicators.iter().map(|adjudicator| {
                        Self::draw_adjudicator_from_uuid(*adjudicator, &participants_by_id)
                    }).collect(),
                    president: debate_ballot.president.map(|president| Self::draw_adjudicator_from_uuid(president, &participants_by_id)),
                }
            }
        ).collect();

        Ok(DrawView { round_uuid: round_uuid, debates })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum View {
    Draw{uuid: u64}
}


#[tauri::command]
fn subscribe_to_view(view: View, db: State<DatabaseConnection>) -> String {
    //format!("Hello, {}! You've been greeted from Rust!", name)
    let draw_view = block_on(DrawView::load(db.inner(), Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())).unwrap();
    println!("{}", serde_json::to_string(&draw_view).unwrap());
    serde_json::to_string(&draw_view).unwrap()
}

fn main() {
    let db = block_on(connect_db()).unwrap();

    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .invoke_handler(tauri::generate_handler![subscribe_to_view])
        .manage(db)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
