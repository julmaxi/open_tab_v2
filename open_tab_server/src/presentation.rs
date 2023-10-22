use std::collections::HashMap;

use axum::{extract::{State, Path}, Json, Router, routing::{post, get}};
use hyper::StatusCode;
use itertools::{Itertools, izip};
use open_tab_entities::{schema, domain::{self, entity::LoadEntity}};
use sea_orm::{prelude::Uuid, EntityTrait, QueryFilter, DatabaseConnection, ColumnTrait};
use serde::{Serialize, Deserialize};

use crate::{response::{APIError, handle_error}, state::AppState};


#[derive(Debug, Serialize, Deserialize, Clone)]
struct DrawPresentationInfo {
    round_id: Uuid,
    round_name: String,

    debates: Vec<DebatePresentationInfo>
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct VenueInfo {
    venue_id: Uuid,
    venue_name: String,
}

impl From<domain::tournament_venue::TournamentVenue> for VenueInfo {
    fn from(venue: domain::tournament_venue::TournamentVenue) -> Self {
        Self {
            venue_id: venue.uuid,
            venue_name: venue.name
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DebatePresentationInfo {
    debate_id: Uuid,
    debate_index: i32,
    venue: Option<VenueInfo>,

    government: TeamPresentationInfo,
    opposition: TeamPresentationInfo,

    adjudicators: Vec<ParticipantPresentationInfo>,
    president: Option<ParticipantPresentationInfo>,

    non_aligned_speakers: Vec<ParticipantPresentationInfo>,
}


impl DebatePresentationInfo {
    fn from_models(
        debate: schema::tournament_debate::Model,
        ballot: domain::ballot::Ballot,
        venues: &HashMap<Uuid, VenueInfo>,
        participants: &HashMap<Uuid, ParticipantPresentationInfo>,
        teams: &HashMap<Uuid, TeamPresentationInfo>
    ) -> Self {
        let non_aligned_speakers = ballot.speeches.iter()
            .filter(|s| s.role == domain::ballot::SpeechRole::NonAligned)
            .map(|s| match s.speaker {
                Some(id) => participants.get(&id).cloned().unwrap_or_default(),
                None => ParticipantPresentationInfo::default()
            }).collect_vec()
        ;

        Self {
            debate_id: debate.uuid,
            debate_index: debate.index,
            venue: debate.venue_id.map(|v: Uuid| venues.get(&v).cloned().unwrap_or_default()),

            government: teams.get(&ballot.government.team.unwrap()).cloned().unwrap_or_default(),
            opposition: teams.get(&ballot.opposition.team.unwrap()).cloned().unwrap_or_default(),

            adjudicators: ballot.adjudicators.iter().map(|id| participants.get(&id).cloned().unwrap_or_default()).collect_vec(),
            president: ballot.president.map(|p| participants.get(&p).cloned().unwrap_or_default()),

            non_aligned_speakers: non_aligned_speakers,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct ParticipantPresentationInfo {
    participant_id: Uuid,
    participant_name: String,
}

impl From<domain::participant::Participant> for ParticipantPresentationInfo {
    fn from(participant: domain::participant::Participant) -> Self {
        Self {
            participant_id: participant.uuid,
            participant_name: participant.name
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct TeamPresentationInfo {
    team_id: Uuid,
    team_name: String,
}

impl From<domain::team::Team> for TeamPresentationInfo {
    fn from(team: domain::team::Team) -> Self {
        Self {
            team_id: team.uuid,
            team_name: team.name
        }
    }
}


async fn get_draw_presentation(
    State(db): State<DatabaseConnection>,
    Path(round_id): Path<Uuid>,
) -> Result<Json<DrawPresentationInfo>, APIError> {
    let round: Option<schema::tournament_round::Model> = schema::tournament_round::Entity::find_by_id(round_id).one(&db).await.map_err(handle_error)?;

    if !round.is_some() {
        return Err(APIError::from((StatusCode::NOT_FOUND, "Round not found")))
    }

    let round = round.unwrap();

    let debates = schema::tournament_debate::Entity::find().filter(
        schema::tournament_debate::Column::RoundId.eq(round_id)
    ).all(&db).await.map_err(handle_error)?;

    let ballot_ids = debates.iter().map(|d| d.ballot_id).collect_vec();
    let ballots = domain::ballot::Ballot::get_many(&db, ballot_ids).await?;

    let participants = domain::participant::Participant::get_all_in_tournament(&db, round.tournament_id).await.map_err(handle_error)?;
    let participants_by_id = participants.into_iter().map(|p| (p.uuid, p.into())).collect::<HashMap<Uuid, ParticipantPresentationInfo>>();

    let teams = domain::team::Team::get_all_in_tournament(&db, round.tournament_id).await?;
    let teams_by_id = teams.into_iter().map(|t| (t.uuid, t.into())).collect::<HashMap<Uuid, TeamPresentationInfo>>();

    let venue_ids = debates.iter().filter_map(|d| d.venue_id).collect_vec();
    let venues_by_ids = domain::tournament_venue::TournamentVenue::get_many(&db, venue_ids).await?
    .into_iter().map(|v| (v.uuid, v.into())).collect();

    let debates = izip![
        debates.into_iter(),
        ballots.into_iter()
    ].map(|(debate, ballot)| {
        DebatePresentationInfo::from_models(debate, ballot, &venues_by_ids, &participants_by_id, &teams_by_id)
    }).sorted_by_key(|i| i.debate_index).collect_vec();

    Ok(Json(
        DrawPresentationInfo {
            round_id: round_id,
            round_name: format!("{}", round.index + 1),
            debates
        }
    ))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionPresentationInfo {
    motion: String,
    info_slide: Option<String>
}

impl From<schema::tournament_round::Model> for MotionPresentationInfo {
    fn from(round: schema::tournament_round::Model) -> Self {
        Self {
            motion: round.motion.unwrap_or("<No motion>".into()),
            info_slide: round.info_slide
        }
    }
}

async fn get_round_motion(
    State(db): State<DatabaseConnection>,
    Path(round_id): Path<Uuid>,
) -> Result<Json<MotionPresentationInfo>, APIError> {
    let round: Option<schema::tournament_round::Model> = schema::tournament_round::Entity::find_by_id(round_id).one(&db).await.map_err(handle_error)?;

    if !round.is_some() {
        return Err(APIError::from((StatusCode::NOT_FOUND, "Round not found")))
    }

    let round = round.unwrap();
    
    Ok(
        Json(round.into())
    )
}


pub fn router() -> Router<AppState> {
    Router::new()
    .route("/draw/:round_id", get(get_draw_presentation))
    .route("/draw/:round_id/motion", get(get_round_motion))
}
