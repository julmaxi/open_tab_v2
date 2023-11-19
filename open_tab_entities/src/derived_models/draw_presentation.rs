use itertools::{Itertools, izip};
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use crate::{domain::{self, entity::LoadEntity}, schema};

use std::collections::HashMap;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrawPresentationInfo {
    pub round_id: Uuid,
    pub round_name: String,
    pub round_index: u32,

    pub motion: String,
    pub info_slide: Option<String>,

    pub debates: Vec<DebatePresentationInfo>
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadDrawError {
    #[error("Round not found")]
    NotFound,
    #[error(transparent)]
    DbError(#[from] sea_orm::DbErr),
    #[error(transparent)]
    ParticpantParseError(#[from] domain::participant::ParticipantParseError),
    #[error(transparent)]
    Other(#[from] anyhow::Error)
}

impl DrawPresentationInfo {
    pub async fn load_for_round<C>(
        db: &C,
        round_id: Uuid
    ) -> Result<Self, LoadDrawError> where C: ConnectionTrait {
        let round: Option<schema::tournament_round::Model> = schema::tournament_round::Entity::find_by_id(round_id).one(db).await?;

        if !round.is_some() {
            return Err(LoadDrawError::NotFound)
        }
    
        let round = round.unwrap();
    
        let debates = schema::tournament_debate::Entity::find().filter(
            schema::tournament_debate::Column::RoundId.eq(round_id)
        ).all(db).await?;
    
        let ballot_ids = debates.iter().map(|d| d.ballot_id).collect_vec();
        let ballots = domain::ballot::Ballot::get_many(db, ballot_ids).await?;
        let institutions_by_id = domain::tournament_institution::TournamentInstitution::get_all_in_tournament(db, round.tournament_id).await?.into_iter().map(|i| (i.uuid, i.into())).collect::<HashMap<Uuid, InstitutionPresentationInfo>>();


        let participants = domain::participant::Participant::get_all_in_tournament(db, round.tournament_id).await?;
        //TODO: We can be more efficient here
        let participants_by_id = participants.iter().map(|p| (p.uuid, ParticipantPresentationInfo::from_participant(p.clone(), &institutions_by_id))).collect::<HashMap<Uuid, _>>();
        let participants_by_team_id = participants.into_iter().filter_map(
            |p| match &p.role {
                domain::participant::ParticipantRole::Speaker(speaker_info) => Some((speaker_info.team_id.clone(), ParticipantPresentationInfo::from_participant(p.clone(), &institutions_by_id))),
                _ => None
            }
        ).filter_map(|(team_id, info)| match team_id {
            Some(team_id) => Some((team_id, info)),
            None => None
        }).group_by(|(team_id, _)| *team_id).into_iter().map(|(team_id, group)| (team_id, group.into_iter().map(|(_, p)| p).collect_vec())).collect::<HashMap<Uuid, Vec<ParticipantPresentationInfo>>>();

        let teams = domain::team::Team::get_all_in_tournament(db, round.tournament_id).await?;
        let teams_by_id = teams.into_iter().map(|t| (t.uuid, TeamPresentationInfo::from_team(t, &participants_by_team_id))).collect::<HashMap<Uuid, TeamPresentationInfo>>();
    
        let venue_ids = debates.iter().filter_map(|d| d.venue_id).collect_vec();
        let venues_by_ids = domain::tournament_venue::TournamentVenue::get_many(db, venue_ids).await?
        .into_iter().map(|v| (v.uuid, v.into())).collect();
    
        let debates = izip![
            debates.into_iter(),
            ballots.into_iter()
        ].map(|(debate, ballot)| {
            DebatePresentationInfo::from_models(debate, ballot, &venues_by_ids, &participants_by_id, &teams_by_id)
        }).sorted_by_key(|i| i.debate_index).collect_vec();

        Ok(Self {
            round_id: round_id,
            round_index: round.index as u32,
            round_name: format!("{}", round.index + 1),
            debates,
            motion: round.motion.unwrap_or("<No motion>".into()),
            info_slide: round.info_slide
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct VenueInfo {
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
pub struct DebatePresentationInfo {
    pub debate_id: Uuid,
    pub debate_index: i32,
    pub venue: Option<VenueInfo>,

    pub government: TeamPresentationInfo,
    pub opposition: TeamPresentationInfo,

    pub adjudicators: Vec<ParticipantPresentationInfo>,
    pub president: Option<ParticipantPresentationInfo>,

    pub non_aligned_speakers: Vec<ParticipantPresentationInfo>,
}


impl DebatePresentationInfo {
    pub fn from_models(
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
pub struct ParticipantPresentationInfo {
    pub participant_id: Uuid,
    pub participant_name: String,
    pub institutions: Vec<InstitutionPresentationInfo>
}

impl ParticipantPresentationInfo {
    fn from_participant(participant: domain::participant::Participant, all_institutions: &HashMap<Uuid, InstitutionPresentationInfo> ) -> Self {
        Self {
            participant_id: participant.uuid,
            participant_name: participant.name,
            institutions: participant.institutions.into_iter().map(|i| 
                all_institutions.get(&i.uuid).map(|i| i.clone()).unwrap_or_default()
            ).collect_vec()
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct TeamPresentationInfo {
    pub team_id: Uuid,
    pub team_name: String,

    pub members: Vec<ParticipantPresentationInfo>,
    pub all_institutions: Vec<InstitutionPresentationInfo>
}
impl TeamPresentationInfo {
    fn from_team(team: crate::prelude::Team, participants_by_team_id: &HashMap<Uuid, Vec<ParticipantPresentationInfo>>) -> TeamPresentationInfo {
        let members = participants_by_team_id.get(&team.uuid).cloned().unwrap_or_default();
        let mut all_institutions = members.iter().flat_map(
            |m| {
                let institutions = m.institutions.clone();
                institutions
            }).unique_by(|i| i.institution_id).sorted_by(|a, b| String::cmp(&a.institution_name, &b.institution_name)).collect_vec();
        Self {
            team_id: team.uuid,
            team_name: team.name,
            members,
            all_institutions
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InstitutionPresentationInfo {
    pub institution_id: Uuid,
    pub institution_name: String,
}

impl From<domain::tournament_institution::TournamentInstitution> for InstitutionPresentationInfo {
    fn from(institution: domain::tournament_institution::TournamentInstitution) -> Self {
        Self {
            institution_id: institution.uuid,
            institution_name: institution.name
        }
    }
}


use rand::Rng;


// Mock Functions
fn mock_venue_info() -> VenueInfo {
    VenueInfo {
        venue_id: Uuid::new_v4(),
        venue_name: "Mock Venue".to_string(),
    }
}

fn mock_institution_presentation_info() -> InstitutionPresentationInfo {
    InstitutionPresentationInfo {
        institution_id: Uuid::new_v4(),
        institution_name: "Mock Institution".to_string(),
    }
}

fn mock_participant_presentation_info() -> ParticipantPresentationInfo {
    ParticipantPresentationInfo {
        participant_id: Uuid::new_v4(),
        participant_name: "Mock Participant".to_string(),
        institutions: vec![mock_institution_presentation_info()],
    }
}

fn mock_team_presentation_info() -> TeamPresentationInfo {
    TeamPresentationInfo {
        team_id: Uuid::new_v4(),
        team_name: "Mock Team".to_string(),
        members: vec![mock_participant_presentation_info(), mock_participant_presentation_info()],
        all_institutions: vec![mock_institution_presentation_info()],
    }
}

fn mock_debate_presentation_info(debate_index: i32) -> DebatePresentationInfo {
    DebatePresentationInfo {
        debate_id: Uuid::new_v4(),
        debate_index,
        venue: Some(mock_venue_info()),
        government: mock_team_presentation_info(),
        opposition: mock_team_presentation_info(),
        adjudicators: vec![mock_participant_presentation_info(), mock_participant_presentation_info()],
        president: Some(mock_participant_presentation_info()),
        non_aligned_speakers: vec![mock_participant_presentation_info()],
    }
}

pub fn mock_draw_presentation_info() -> DrawPresentationInfo {
    DrawPresentationInfo {
        round_id: Uuid::new_v4(),
        round_index: 1,
        round_name: "Mock Round".to_string(),
        motion: "This house would use mock data".to_string(),
        info_slide: Some("Mock Info Slide".to_string()),
        debates: (1..=3).map(|index| mock_debate_presentation_info(index)).collect(),
    }
}

