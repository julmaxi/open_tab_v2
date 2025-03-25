use std::collections::HashMap;

use crate::domain::tournament_institution::TournamentInstitution;


use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};
use crate::prelude::*;


use itertools::Itertools;


#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentParticipantsInfo {
    pub participants_by_id: HashMap<Uuid, Participant>,
    pub teams_by_id: HashMap<Uuid, Team>,
    pub team_members: HashMap<Uuid, Vec<Uuid>>,
    pub speaker_teams: HashMap<Uuid, Uuid>,
    pub institutions_by_id: HashMap<Uuid, TournamentInstitution>,
}

impl TournamentParticipantsInfo {
    pub fn new() -> Self {
        Self {
            participants_by_id: HashMap::new(),
            teams_by_id: HashMap::new(),
            team_members: HashMap::new(),
            speaker_teams: HashMap::new(),
            institutions_by_id: HashMap::new(),
        }
    }
    pub fn get_adjudicators(&self) -> Vec<&Participant> {
        self.participants_by_id.values().filter(|participant| {
            if let ParticipantRole::Adjudicator(_) = &participant.role {
                true
            }
            else {
                false
            }
        }).collect()
    }
}

impl TournamentParticipantsInfo {
    pub async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let all_participants = Participant::get_all_in_tournament(db, tournament_id).await?;
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
        let teams_by_id = Team::get_all_in_tournament(db, tournament_id).await?.into_iter().map(|team| (team.uuid, team)).collect::<HashMap<_, _>>();
        let participants_by_id = all_participants.into_iter().map(|speaker| (speaker.uuid, speaker)).collect::<HashMap<_, _>>();
        let institutions_by_id = TournamentInstitution::get_all_in_tournament(db, tournament_id).await?.into_iter().map(|inst| (inst.uuid, inst)).collect::<HashMap<_, _>>();

        let speaker_teams = team_members.iter().flat_map(|(team_id, speakers)| {
            speakers.iter().map(move |speaker_id| (*speaker_id, *team_id))
        }).collect::<HashMap<_, _>>();

        Ok(Self {
            participants_by_id,
            teams_by_id,
            team_members,
            speaker_teams,
            institutions_by_id
        })
    }
}
