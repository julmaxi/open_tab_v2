use itertools::izip;
use open_tab_entities::schema;
use sea_orm::ActiveValue;
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
    pub role: ParticipantRole,
    pub clashes: Vec<Clash>,
    pub institutions: Vec<Institution>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clash {
    pub target_uuid: Uuid,
    pub target_name: String,
    pub clash_strength: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Institution {
    pub uuid: Uuid,
    pub name: String,
    pub clash_strength: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParticipantRole {
    Speaker{team_id: Uuid},
    Adjudicator{chair_skill: i16, panel_skill: i16}
}


impl ParticipantsListView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<ParticipantsListView, Box<dyn Error>> where C: ConnectionTrait {
       let participants = domain::participant::Participant::get_all_in_tournament(db, tournament_uuid).await?;

       let all_institutions: Vec<Vec<_>> = participants.iter().map(|p| schema::participant::Model {
            uuid: p.uuid,
            tournament_id: Uuid::nil(),
            name: "".into(),
        }).collect_vec().load_many(schema::participant_tournament_institution::Entity, db).await?;

        let institution_ids = all_institutions.iter().flatten().map(|i| i.institution_id).collect_vec();
        let institution_names = schema::tournament_institution::Entity::find().
            filter(
                schema::tournament_institution::Column::Uuid.is_in(institution_ids)
            ).all(db).await?.into_iter().map(|i| (i.uuid, i.name)).collect::<HashMap<_, _>>();
        
        let participants = izip![participants, all_institutions].collect_vec();

       let (adjudicators, team_members) : (Vec<_>, Vec<_>) = participants.into_iter().partition(|p| 
        match p.0.role {
            domain::participant::ParticipantRole::Adjudicator(_) => true,
            _ => false
        });

        let adjudicators = adjudicators.into_iter().filter_map(|(p, institutions)| {
            match p.role {
                domain::participant::ParticipantRole::Adjudicator(
                    Adjudicator { chair_skill, panel_skill }
                ) => Some(ParticipantEntry {
                    uuid: p.uuid,
                    name: p.name,
                    role: ParticipantRole::Adjudicator {
                        chair_skill,
                        panel_skill
                    },
                    institutions: institutions.into_iter().map(
                        |i| Institution {
                            uuid: i.institution_id,
                            name: institution_names.get(&i.institution_id).unwrap_or(&"Unknown Institution".to_string()).clone(),
                            clash_strength: i.clash_strength
                        }
                    ).collect_vec(),
                    clashes: vec![]
                }),
                _ => None
            }
        }).collect_vec();

        let teams = team_members.into_iter().filter_map(
            |(p, institutions)| match p.role {
                domain::participant::ParticipantRole::Speaker(
                    Speaker { team_id }
                ) => {
                    if let Some(team_id) = team_id {
                        Some((team_id, ParticipantEntry {
                            uuid: p.uuid,
                            name: p.name,
                            role: ParticipantRole::Speaker { team_id  },
                            institutions: institutions.into_iter().map(
                                |i| Institution {
                                    uuid: i.institution_id,
                                    name: institution_names.get(&i.institution_id).unwrap_or(&"Unknown Institution".to_string()).clone(),
                                    clash_strength: i.clash_strength
                                }
                            ).collect_vec(),
                            clashes: vec![]
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