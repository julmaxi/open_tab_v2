use sea_orm::prelude::Uuid;
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;

use open_tab_entities::domain;

use itertools::Itertools;

use crate::LoadedView;

pub struct LoadedParticipantsListView {
    pub view: ParticipantsListView,
    pub tournament_id: Uuid
    //TODO: Use this to cache team and participant names
    //to avoid a full reload every time
    //Alternatively, it would be interesting to try to implement
    //dependent views.
}

impl LoadedParticipantsListView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<LoadedParticipantsListView, Box<dyn Error>> where C: ConnectionTrait {
        Ok(
            LoadedParticipantsListView {
                tournament_id: tournament_uuid,
                view: ParticipantsListView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedParticipantsListView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroups) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.participants.len() > 0 || changes.teams.len() > 0 {
            self.view = ParticipantsListView::load_from_tournament(db, self.tournament_id).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantsListView {
    pub adjudicators: HashMap<Uuid, ParticipantEntry>,
    pub teams: HashMap<Uuid, TeamEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamEntry {
    pub uuid: Uuid,
    pub name: String,
    pub members: HashMap<Uuid, ParticipantEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantEntry {
    pub uuid: Uuid,
    pub name: String,
    #[serde(flatten)]
    pub role: ParticipantRole
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParticipantRole {
    Speaker{team_id: Uuid},
    Adjudicator{}
}


impl ParticipantsListView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<ParticipantsListView, Box<dyn Error>> where C: ConnectionTrait {
       let participants = domain::participant::Participant::get_all_in_tournament(db, tournament_uuid).await?;

       let (adjudicators, team_members) : (Vec<_>, Vec<_>) = participants.into_iter().partition(|p| 
        match p.role {
            domain::participant::ParticipantRole::Adjudicator(_) => true,
            _ => false
        });

        let adjudicators = adjudicators.into_iter().map(|p| {
            ParticipantEntry {
                uuid: p.uuid,
                name: p.name,
                role: ParticipantRole::Adjudicator {  }
            }
        }).collect_vec();

        let teams = team_members.into_iter().filter_map(
            |p| match p.role {
                domain::participant::ParticipantRole::Speaker(
                    Speaker { team_id }
                ) => {
                    if let Some(team_id) = team_id {
                        Some((team_id, ParticipantEntry {
                            uuid: p.uuid,
                            name: p.name,
                            role: ParticipantRole::Speaker { team_id  }
                        }))
                    }
                    else {
                        None
                    }
                },
                _ => None
            }
        ).into_group_map();
        
        let team_names = domain::team::Team::get_all_in_tournament(db, tournament_uuid).await?.into_iter(
        ).map(|t| (t.uuid, t.name)).collect::<HashMap<_, _>>();   

        let teams = teams.into_iter().map(|(team_id, speakers)| {
            TeamEntry {
                uuid: team_id,
                name: team_names.get(&team_id).unwrap().clone(),
                members: speakers.into_iter().map(|s| (s.uuid, s)).collect()
            }
        }).collect_vec();

        Ok(
            ParticipantsListView {
                adjudicators: adjudicators.into_iter().map(|a| (a.uuid, a)).collect(),
                teams: teams.into_iter().map(|t| (t.uuid, t)).collect()
            }
        )
    }
}