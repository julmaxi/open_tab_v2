use std::fmt::Display;
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;

use open_tab_entities::schema::{self, tournament_round};

use itertools::izip;
use itertools::Itertools;


#[async_trait]
pub trait LoadedView : Sync + Send {
    // We can't use a connection trait here, since otherwise the trait is not object safe
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroups) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>>;
    async fn view_string(&self) -> Result<String, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct TournamentParticipantsInfo {
    pub participants_by_id: HashMap<Uuid, Participant>,
    pub teams_by_id: HashMap<Uuid, Team>,
    pub team_members: HashMap<Uuid, Vec<Uuid>>,
    pub speaker_teams: HashMap<Uuid, Uuid>,
}

impl TournamentParticipantsInfo {
    pub async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
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

        let speaker_teams = team_members.iter().flat_map(|(team_id, speakers)| {
            speakers.iter().map(move |speaker_id| (*speaker_id, *team_id))
        }).collect::<HashMap<_, _>>();

        Ok(Self {
            participants_by_id,
            teams_by_id,
            team_members,
            speaker_teams
        })
    }
}