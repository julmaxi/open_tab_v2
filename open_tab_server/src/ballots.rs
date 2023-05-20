
use std::collections::hash_map::RandomState;
use std::{collections::HashMap, error::Error};

use open_tab_entities::domain::ballot;
use open_tab_entities::domain::entity::LoadEntity;
use open_tab_entities::prelude::*;
use open_tab_entities::{Entity, domain};
use open_tab_entities::domain::{ballot::Ballot};
use open_tab_entities::schema::{self};

use rocket::futures::TryFutureExt;
use rocket::http::hyper::body::HttpBody;
use rocket::request::FromRequest;
use rocket::response::status::{Custom, self};
use rocket::serde::{Deserialize, Serialize, json::Json};
use migration::{JoinType};
use rocket::{State, get, post, routes, Route, Request};

use sea_orm::{prelude::*, ConnectionTrait, QuerySelect};
use itertools::Itertools;
use rocket::http::Status;

use base64::{Engine as _, engine::general_purpose};



use crate::handle_error_dyn;
use super::handle_error;


struct LoggedInParticipant {
    participant_id: Uuid,
}

async fn parse_participant_header(header: &str, db: &DatabaseConnection) -> Result<Option<LoggedInParticipant>, Box<dyn Error>> {
    let parts = header.split(" ").collect::<Vec<_>>();

    if parts.len() == 2 {
        let auth_str = general_purpose::URL_SAFE.decode(parts[1].as_bytes())?;
        let auth_str = String::from_utf8(auth_str)?;
        let parts = auth_str.split(":").collect::<Vec<_>>();
        if parts.len() == 2 {
            let participant_id : Uuid = parts[0].parse()?;

            let participant = open_tab_entities::schema::participant::Entity::find_by_id(
                participant_id
            ).one(db).await?;

            if let Some(participant) = participant {
                let registration_key = general_purpose::URL_SAFE.decode(parts[1].as_bytes())?;
                if participant.registration_key == Some(registration_key) {
                    Ok(Some(LoggedInParticipant {
                        participant_id: participant_id,
                    }))
                }
                else {
                    Ok(None)
                }
            }
            else {
                Ok(None)
            }

        }
        else {
            Err("Too many parts".into())
        }
    }
    else {
        Err("Too short".into())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for LoggedInParticipant {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let header = request.headers().get_one("Authorization");
        let db = request.rocket().state::<DatabaseConnection>().unwrap();
        
        let participant = match header {
            Some(header) => match parse_participant_header(header, db).await {
                Ok(participant) => participant,
                Err(err) => {
                    eprintln!("Error parsing participant header: {}", err);
                    None
                }
            },
            None => None
        };
        match participant {
            Some(participant) => rocket::outcome::Outcome::Success(participant),
            None => rocket::outcome::Outcome::Failure((Status::Unauthorized, ()))
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct DisplayBallot {
    uuid: uuid::Uuid,

    adjudicators: Vec<DisplayAdjudicator>,
    president: Option<DisplayAdjudicator>,
    government: DisplayBallotTeam,
    opposition: DisplayBallotTeam,

    speeches: Vec<DisplayBallotSpeech>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DisplayAdjudicator {
    uuid: uuid::Uuid,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct DisplayBallotTeam {
    uuid: uuid::Uuid,
    name: String,
    members: Vec<DisplaySpeaker>,
    scores: HashMap<Uuid, i16>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DisplaySpeaker {
    uuid: uuid::Uuid,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DisplayBallotSpeech {
    scores: HashMap<Uuid, i16>,
    speaker: Option<DisplaySpeaker>,
    position: u8,
    role: SpeechRole
}

impl DisplayBallot {
    async fn from_id<C>(ballot_id: Uuid, db: &C) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        let ballot = Ballot::get(db, ballot_id).await?;

        let teams = ballot.government.team.iter().chain(ballot.opposition.team.iter()).map(|u| *u).collect_vec();
    
        let team_name_map = if teams.len() > 0 {
            let teams = schema::team::Entity::find()
            .filter(schema::team::Column::Uuid.is_in(teams.clone()))
            .all(db)
            .await?;
            teams.into_iter().map(|t| (t.uuid, t.name)).collect()
        }
        else {
            HashMap::new()
        };
    
        let team_members = schema::participant::Entity::find()
            .find_also_related(schema::speaker::Entity)
            .filter(schema::speaker::Column::TeamId.is_in(teams.clone()))
            .all(db)
            .await?;
    
        let adjudicators = schema::participant::Entity::find()
            .filter(schema::participant::Column::Uuid.is_in(ballot.adjudicators.clone()))
            .all(db)
            .await?;
    
        let speech_speakers = schema::participant::Entity::find()
            .join_rev(
                JoinType::InnerJoin,
                schema::ballot_speech::Entity::belongs_to(schema::participant::Entity).from(schema::ballot_speech::Column::SpeakerId).to(schema::participant::Column::Uuid)
                .into(),
            )
            .filter(schema::ballot_speech::Column::BallotId.eq(ballot_id).and(
                schema::ballot_speech::Column::SpeakerId.is_in(ballot.speeches.iter().filter_map(|s| s.speaker))
            ))
            .all(db)
            .await?;
    
        let participant_name_map : HashMap<Uuid, String, RandomState> = HashMap::from_iter(
            team_members.iter().map(|p| (p.0.uuid, p.0.name.clone())).chain(
                speech_speakers.iter().map(|p| (p.uuid, p.name.clone()))
            ).chain(
                adjudicators.iter().map(|a| (a.uuid, a.name.clone()))
            )
        );
    
        let gov_members = team_members.iter().filter(
            |p| {
                if let Some(gov_id) = ballot.government.team {
                    p.1.as_ref().map(|p| p.team_id).flatten().unwrap_or(Uuid::nil()) == gov_id
                }
                else {
                    false
                }
            }
        ).map(|p| DisplaySpeaker{uuid: p.0.uuid, name: p.0.name.clone()}).collect_vec();
        let opp_members = team_members.iter().filter(
            |p| {
                if let Some(opp_id) = ballot.opposition.team {
                    p.1.as_ref().map(|p| p.team_id).flatten().unwrap_or(Uuid::nil()) == opp_id
                }
                else {
                    false
                }
            }
        ).map(|p| DisplaySpeaker{uuid: p.0.uuid, name: p.0.name.clone()}).collect_vec();
    
        Ok(DisplayBallot {
            uuid: ballot.uuid,
            adjudicators: ballot.adjudicators.iter().map(|a| DisplayAdjudicator{uuid: *a, name: participant_name_map.get(&a).unwrap_or(&"Unknown".to_string()).clone()}).collect_vec(),
            government: ballot.government.team.as_ref().map(
                |team_id| DisplayBallotTeam {
                    uuid: *team_id,
                    name: team_name_map.get(team_id).unwrap_or(&"Unknown".to_string()).clone(),
                    members: gov_members,
                    scores: ballot.government.scores.iter().map(|(adj, score)| {
                        (*adj, score.total())
                    }).collect(),
                }
            ).unwrap_or(Default::default()),
            opposition: ballot.opposition.team.as_ref().map(
                |team_id| DisplayBallotTeam {
                    uuid: *team_id,
                    name: team_name_map.get(team_id).unwrap_or(&"Unknown".to_string()).clone(),
                    members: opp_members,
                    scores: ballot.opposition.scores.iter().map(|(adj, score)| {
                        (*adj, score.total())
                    }).collect(),
                }
            ).unwrap_or(Default::default()),
            speeches: ballot.speeches.iter().map(|s| {
                DisplayBallotSpeech {
                    scores: s.scores.iter().map(|(adj, score)| {
                        (*adj, score.total())
                    }).collect(),
                    speaker: s.speaker.as_ref().map(|speaker_id| DisplaySpeaker {
                        uuid: *speaker_id,
                        name: participant_name_map.get(speaker_id).unwrap_or(&"Unknown".to_string()).clone(),
                    }),
                    role: s.role.clone(),
                    position: s.position,
                }
            }).collect_vec(),
            president: ballot.president.as_ref().map(|president_id| DisplayAdjudicator {
                uuid: *president_id,
                name: participant_name_map.get(president_id).unwrap_or(&"Unknown".to_string()).clone(),
            }),
        })
    }
}

#[get("/ballot/<ballot_id>")]
async fn get_ballot(
    db: &State<DatabaseConnection>,
    ballot_id: rocket::serde::uuid::Uuid) -> Result<Json<DisplayBallot>, Custom<String>> {

    let display_ballot = DisplayBallot::from_id(ballot_id, db.inner()).await.map_err(handle_error_dyn)?;

    Ok(Json(display_ballot))
}

#[derive(Debug, Serialize, Deserialize)]
struct SubmissionResponse {
    uuid: Uuid,
    debate_id: Uuid,
    ballot: DisplayBallot,
}

#[get("/ballot-submission/<submission_id>")]
async fn get_ballot_submission(
    db: &State<DatabaseConnection>,
    submission_id: rocket::serde::uuid::Uuid) -> Result<Json<SubmissionResponse>, Custom<String>> {
    
    let submission = schema::debate_backup_ballot::Entity::find_by_id(submission_id).one(db.inner()).await.map_err(handle_error)?;

    if let Some(submission) = submission {
        let ballot = DisplayBallot::from_id(submission.ballot_id, db.inner()).await.map_err(handle_error_dyn)?;

        Ok(Json(SubmissionResponse { uuid: submission_id, ballot, debate_id: submission.debate_id }))    
    }
    else {
        return Err(Custom(Status::NotFound, "Submission not found".to_string()));
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct DebateInfo {
    uuid: Uuid,
    ballot: DisplayBallot
}

#[get("/debate/<debate_id>")]
async fn get_debate(
    db: &State<DatabaseConnection>,
    debate_id: rocket::serde::uuid::Uuid,
    user: LoggedInParticipant
) -> Result<Json<DebateInfo>, Custom<String>> {
    
    let debate = schema::tournament_debate::Entity::find()
        .filter(schema::tournament_debate::Column::Uuid.eq(debate_id))
        .one(db.inner())
        .await
        .map_err(handle_error)?.ok_or(Custom(Status::InternalServerError, "Debate not found".to_string()))?;

    let display_ballot = DisplayBallot::from_id(debate.ballot_id, db.inner()).await.map_err(handle_error_dyn)?;

    Ok(Json(
        DebateInfo {
            uuid: debate_id,
            ballot: display_ballot
        }
    ))
}

#[get("/debate/<debate_id>/ballot")]
async fn get_debate_current_ballot(
    db: &State<DatabaseConnection>,
    debate_id: rocket::serde::uuid::Uuid) -> Result<Json<DisplayBallot>, Custom<String>> {
    
    let debate = schema::tournament_debate::Entity::find()
        .filter(schema::tournament_debate::Column::Uuid.eq(debate_id))
        .one(db.inner())
        .await
        .map_err(handle_error)?.ok_or(Custom(Status::InternalServerError, "Ballot not found".to_string()))?;

    let display_ballot = DisplayBallot::from_id(debate.ballot_id, db.inner()).await.map_err(handle_error_dyn)?;

    Ok(Json(display_ballot))
}


#[derive(Serialize, Deserialize, Debug)]
struct CreateDebateBallotResponse {
    debate_ballot_uuid: Uuid
}

/*
#[post("/debate/<debate_id>/ballots", data = "<ballot>")]
async fn submit_ballot(
    db: &State<DatabaseConnection>,
    debate_id: rocket::serde::uuid::Uuid,
    ballot: Json<DisplayBallot>,
) -> Result<Json<CreateDebateBallotResponse>, Custom<String>> {
    let db = db.inner();
    let tournament = schema::tournament::Entity::find()
    .inner_join(schema::tournament_round::Entity)
    .join(JoinType::InnerJoin, schema::tournament_round::Relation::TournamentDebate.def())
    .filter(schema::tournament_debate::Column::Uuid.eq(debate_id)).one(db).await.map_err(handle_error)?;

    let debate = schema::tournament_debate::Entity::find()
        .filter(schema::tournament_debate::Column::Uuid.eq(debate_id))
        .one(db)
        .await
        .map_err(handle_error)?.ok_or(Custom(Status::InternalServerError, "Debate not found".to_string()))?;

    let tournament_id = tournament.ok_or(Custom(Status::NotFound, "Tournament not found".to_string()))?.uuid;
    
    let ballot = ballot.into_inner();

    let base_ballot = domain::ballot::Ballot::get(db, debate.ballot_id).await.map_err(handle_error_dyn)?;

    let new_ballot = ballot::Ballot {
        uuid: Uuid::new_v4(),
        speeches: ballot.speeches.into_iter().map(
            |s| ballot::Speech {
                speaker: s.speaker.map(|s| s.uuid),
                role: s.role,
                position: s.position,
                scores: s.scores.into_iter().map(|(adj, score)| (adj, ballot::SpeakerScore::Aggregate(score))).collect()
            }
        ).collect_vec(),
        government: ballot::BallotTeam {
            team: Some(ballot.government.uuid),
            scores: ballot.government.scores.into_iter().map(|(adj, score)| (adj, ballot::TeamScore::Aggregate(score))).collect()
        },
        opposition: ballot::BallotTeam {
            team: Some(ballot.opposition.uuid),
            scores: ballot.opposition.scores.into_iter().map(|(adj, score)| (adj, ballot::TeamScore::Aggregate(score))).collect()
        },
        adjudicators: ballot.adjudicators.into_iter().map(|a| a.uuid).collect_vec(),
        president: base_ballot.president
    };

    let entry_uuid = Uuid::new_v4();
    let ballot_entry = domain::debate_backup_ballot::DebateBackupBallot {
        uuid: entry_uuid,
        ballot_id: new_ballot.uuid,
        debate_id,
        timestamp: chrono::offset::Local::now().naive_local(),
    };

    let mut groups = open_tab_entities::EntityGroup::new();
    groups.add(Entity::Ballot(new_ballot));
    groups.add(Entity::DebateBackupBallot(ballot_entry));
    groups.save_all_and_log_for_tournament(db, tournament_id).await.map_err(handle_error_dyn)?;

    Ok(Json(CreateDebateBallotResponse{debate_ballot_uuid: entry_uuid}))
}
 */

 #[post("/debate/<debate_id>/ballots", data = "<ballot>")]
 async fn submit_ballot(
     db: &State<DatabaseConnection>,
     debate_id: rocket::serde::uuid::Uuid,
     ballot: Json<Ballot>,
 ) -> Result<Json<CreateDebateBallotResponse>, Custom<String>> {
    let db = db.inner();
    let tournament = schema::tournament::Entity::find()
     .inner_join(schema::tournament_round::Entity)
     .join(JoinType::InnerJoin, schema::tournament_round::Relation::TournamentDebate.def())
     .filter(schema::tournament_debate::Column::Uuid.eq(debate_id)).one(db).await.map_err(handle_error)?;

    if tournament.is_none() {
        return Err(Custom(Status::NotFound, "Debate not found".to_string()));
    }

    let tournament = tournament.unwrap();

    let mut ballot = ballot.into_inner();
    ballot.uuid = Uuid::new_v4();
    let debate_ballot_uuid = Uuid::new_v4();
    let ballot_entry = domain::debate_backup_ballot::DebateBackupBallot {
        uuid: debate_ballot_uuid,
        ballot_id: ballot.uuid,
        debate_id,
        timestamp: chrono::offset::Local::now().naive_local(),
    };

    let mut groups = open_tab_entities::EntityGroup::new();
    groups.add(Entity::Ballot(ballot));
    groups.add(Entity::DebateBackupBallot(ballot_entry));
    groups.save_all_and_log_for_tournament(db, tournament.uuid).await.map_err(handle_error_dyn)?;

    /*
     let debate = schema::tournament_debate::Entity::find()
         .filter(schema::tournament_debate::Column::Uuid.eq(debate_id))
         .one(db)
         .await
         .map_err(handle_error)?.ok_or(Custom(Status::InternalServerError, "Debate not found".to_string()))?;
 
     let tournament_id = tournament.ok_or(Custom(Status::NotFound, "Tournament not found".to_string()))?.uuid;
     
     let ballot = ballot.into_inner();
 
     let base_ballot = domain::ballot::Ballot::get(db, debate.ballot_id).await.map_err(handle_error_dyn)?;
 
     let new_ballot = ballot::Ballot {
         uuid: Uuid::new_v4(),
         speeches: ballot.speeches.into_iter().map(
             |s| ballot::Speech {
                 speaker: s.speaker,
                 role: s.role,
                 position: s.position,
                 scores: s.scores.into_iter().map(|(adj, score)| (adj, ballot::SpeakerScore::Aggregate(score))).collect()
             }
         ).collect_vec(),
         government: ballot::BallotTeam {
             team: Some(ballot.government.uuid),
             scores: ballot.government.scores.into_iter().map(|(adj, score)| (adj, ballot::TeamScore::Aggregate(score))).collect()
         },
         opposition: ballot::BallotTeam {
             team: Some(ballot.opposition.uuid),
             scores: ballot.opposition.scores.into_iter().map(|(adj, score)| (adj, ballot::TeamScore::Aggregate(score))).collect()
         },
         adjudicators: ballot.adjudicators.into_iter().map(|a| a.uuid).collect_vec(),
         president: base_ballot.president
     };
 
     let entry_uuid = Uuid::new_v4();
     let ballot_entry = domain::debate_backup_ballot::DebateBackupBallot {
         uuid: entry_uuid,
         ballot_id: new_ballot.uuid,
         debate_id,
         timestamp: chrono::offset::Local::now().naive_local(),
     };
 
     let mut groups = open_tab_entities::EntityGroup::new();
     groups.add(Entity::Ballot(new_ballot));
     groups.add(Entity::DebateBackupBallot(ballot_entry));
     groups.save_all_and_log_for_tournament(db, tournament_id).await.map_err(handle_error_dyn)?;

      */
 
     Ok(Json(CreateDebateBallotResponse{debate_ballot_uuid: debate_ballot_uuid}))
 }

pub fn routes() -> Vec<Route> {
    routes![get_ballot, get_debate, submit_ballot, get_debate_current_ballot, get_ballot_submission]
}