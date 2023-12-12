use std::collections::HashMap;

use sea_orm::{prelude::Uuid, EntityTrait, QueryFilter, ColumnTrait};

use crate::{domain::participant::{Participant, ParticipantRole, Speaker}, schema};



pub struct ParticipantRegistrationInfo {
    pub name: String,
    pub role: String,
    pub registration_url: Option<String>,
}

pub struct RegistrationInfo {
    pub participant_info: Vec<ParticipantRegistrationInfo>,
}

impl RegistrationInfo {
    pub async fn load_from_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        let participants = Participant::get_all_in_tournament(db, tournament_id).await?;
        let remote_info = schema::tournament_remote::Entity::find().filter(schema::tournament_remote::Column::TournamentId.eq(tournament_id)).one(db).await?;
        let mut target_url = String::new();
        let remote_url = remote_info.map(|r| r.url).map(
            |url| {
                //FIXME: This is a hack to make it work with the current setup
                //but this should be discoverable from the remote
                let parsed_url = url::Url::parse(&url).unwrap();

                if parsed_url.host_str().unwrap() == "localhost" {
                    target_url.extend("http://localhost:5173".chars());
                } else {
                    let root = parsed_url.host_str().unwrap().splitn(2, ".").collect::<Vec<_>>()[1];
                    target_url.extend(format!("https://tabs.{}", root).chars());
                }

                target_url
            }
        );


        
        let team_names_by_id = schema::team::Entity::find()
            .filter(schema::team::Column::TournamentId.eq(tournament_id))
            .all(db)
            .await?
            .into_iter()
            .map(|t| (t.uuid, t.name))
            .collect::<HashMap<_, _>>();

        let participant_info = participants
            .into_iter()
            .map(|p| ParticipantRegistrationInfo {
                name: p.name,
                role: match p.role {
                    ParticipantRole::Adjudicator(..) => "Jury".into(),
                    ParticipantRole::Speaker(Speaker { team_id: Some(team_id) }) => team_names_by_id.get(&team_id).cloned().unwrap_or("".into()),
                    _ => "".into()
                },
                registration_url: match (&remote_url, p.registration_key) {
                    (Some(remote_url), Some(secret)) => {
                        let key = Participant::encode_registration_key(p.uuid, &secret);

                        Some(format!("{}/register/{}", remote_url, key))
                    },
                    _ => None,
                }
            })
            .collect();

        Ok(
            Self {
                participant_info
            }
        )
    }
}