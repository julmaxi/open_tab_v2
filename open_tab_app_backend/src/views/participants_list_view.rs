

use itertools::izip;
use open_tab_entities::EntityType;
use open_tab_entities::schema;

use sea_orm::prelude::Uuid;
use std::collections::HashMap;

use async_trait::async_trait;
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
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<LoadedParticipantsListView, anyhow::Error> where C: ConnectionTrait {
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
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.participants.len() > 0 || changes.teams.len() > 0 || changes.deletions.iter().any(|d| d.0 == EntityType::Participant) || changes.deletions.iter().any(|d| d.0 == EntityType::Team) {
            self.view = ParticipantsListView::load_from_tournament(db, self.tournament_id).await?;

            let mut out: HashMap<String, Json> = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, anyhow::Error> {
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
    pub institutions: Vec<Institution>,
    pub registration_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clash {
    pub participant_uuid: Uuid,
    pub participant_name: String,
    pub clash_severity: i16,
    #[serde(flatten)]
    pub clash_direction: ClashDirection
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "direction")]
pub enum ClashDirection {
    Incoming,
    Outgoing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Institution {
    pub uuid: Uuid,
    pub name: String,
    pub clash_severity: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParticipantRole {
    Speaker{team_id: Uuid},
    Adjudicator{chair_skill: i16, panel_skill: i16, unavailable_rounds: Vec<Uuid>,
    }
}


impl ParticipantsListView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<ParticipantsListView, anyhow::Error> where C: ConnectionTrait {
       let participants = domain::participant::Participant::get_all_in_tournament(db, tournament_uuid).await?;

       // This is a little hack so we can load all the institutions using the Loader trait
       let all_institutions: Vec<Vec<_>> = participants.iter().map(|p| schema::participant::Model {
            uuid: p.uuid,
            tournament_id: Uuid::nil(),
            name: "".into(),
            registration_key: None,
        }).collect_vec().load_many(schema::participant_tournament_institution::Entity, db).await?;

        let all_clashes = schema::participant_clash::Entity::find()
        .filter(
            schema::participant_clash::Column::DeclaringParticipantId.is_in(
                participants.iter().map(|p| p.uuid).collect_vec()
            ).or(
                schema::participant_clash::Column::TargetParticipantId.is_in(
                    participants.iter().map(|p| p.uuid).collect_vec()
                )
            )
        )
        .all(db).await?;

        let outgoing_clashes = all_clashes.iter().into_group_map_by(|c| c.declaring_participant_id);
        let incoming_clashes = all_clashes.iter().into_group_map_by(|c| c.target_participant_id);

        let institution_ids = all_institutions.iter().flatten().map(|i| i.institution_id).collect_vec();
        let institution_names = schema::tournament_institution::Entity::find().
            filter(
                schema::tournament_institution::Column::Uuid.is_in(institution_ids)
            ).all(db).await?.into_iter().map(|i| (i.uuid, i.name)).collect::<HashMap<_, _>>();

        let participant_names = participants.iter().map(|p| (p.uuid, p.name.clone())).collect::<HashMap<_, _>>();
        
        let participants = izip![participants, all_institutions];

        let participants = participants.filter_map(|(p, institutions)| {
            let clashes = itertools::chain!(
                outgoing_clashes.get(&p.uuid).into_iter().flatten().map(|c| (ClashDirection::Outgoing, c)),
                incoming_clashes.get(&p.uuid).into_iter().flatten().map(|c| (ClashDirection::Incoming, c))
            ).map(|(dir, c)| Clash {
                participant_uuid: match dir { ClashDirection::Incoming => c.declaring_participant_id, ClashDirection::Outgoing => c.target_participant_id },
                participant_name: participant_names.get(&match dir {ClashDirection::Incoming => c.declaring_participant_id, ClashDirection::Outgoing => c.target_participant_id } ).unwrap_or(&"Unknown Participant".to_string()).clone(),
                clash_severity: c.clash_severity,
                clash_direction: dir
            }).collect_vec();
            let institutions = institutions.into_iter().map(
                |i| Institution {
                    uuid: i.institution_id,
                    name: institution_names.get(&i.institution_id).unwrap_or(&"Unknown Institution".to_string()).clone(),
                    clash_severity: i.clash_severity
                }
            ).collect_vec();
            match p.role {
                domain::participant::ParticipantRole::Adjudicator(
                    Adjudicator { chair_skill, panel_skill, unavailable_rounds}
                ) => Some(ParticipantEntry {
                    uuid: p.uuid,
                    name: p.name,
                    role: ParticipantRole::Adjudicator {
                        chair_skill,
                        panel_skill,
                        unavailable_rounds
                    },
                    institutions,
                    clashes,
                    registration_key: p.registration_key.map(|k| {
                        Participant::encode_registration_key(p.uuid, &k)
                    }),
                }),
                domain::participant::ParticipantRole::Speaker(
                    Speaker { team_id }
                ) => {
                    if let Some(team_id) = team_id {
                        Some(ParticipantEntry {
                            uuid: p.uuid,
                            name: p.name,
                            role: ParticipantRole::Speaker { team_id  },
                            institutions,
                            clashes,
                            registration_key: p.registration_key.map(|k| Participant::encode_registration_key(p.uuid, &k)),
                        })    
                    }
                    else {
                        None
                    }
                },
            }
        });

        let (adjudicators, team_members) : (Vec<_>, Vec<_>) = participants.into_iter().partition(|p| 
            match p.role {
                ParticipantRole::Adjudicator { .. } => true,
                _ => false
            });
        
        let teams = team_members.into_iter().map(|p| {
            match p.role {
                ParticipantRole::Speaker { team_id } => (team_id, p),
                _ => unreachable!()
            }
        }).into_group_map();
    
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