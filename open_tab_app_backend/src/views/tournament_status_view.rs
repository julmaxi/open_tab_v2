
use open_tab_entities::domain::ballot::Ballot;
use open_tab_entities::domain::entity::LoadEntity;
use open_tab_entities::schema;

use sea_orm::QueryOrder;
use sea_orm::prelude::Uuid;
use std::path::Display;
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;



use itertools::Itertools;

use crate::LoadedView;
use crate::TournamentParticipantsInfo;

pub struct LoadedTournamentStatusView {
    pub view: TournamentStatusView,
    pub tournament_uuid: Uuid
}

impl LoadedTournamentStatusView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        Ok(
            LoadedTournamentStatusView {
                tournament_uuid,
                view: TournamentStatusView::load(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedTournamentStatusView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.tournaments.len() > 0 {
            self.view = TournamentStatusView::load(db, self.tournament_uuid).await?;
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
pub struct TournamentStatusView {
    annoucements_password: Option<String>,
}

impl TournamentStatusView {
    async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, Box<dyn Error>> where C: ConnectionTrait {
        let tournament = Tournament::get(db, tournament_id).await?;
        Ok(Self {
            annoucements_password: tournament.annoucements_password,
        })
    }
}