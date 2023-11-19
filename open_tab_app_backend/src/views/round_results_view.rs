
use open_tab_entities::derived_models::{ResultDebate, BackupBallot};
use open_tab_entities::domain::ballot::Ballot;
use open_tab_entities::domain::entity::LoadEntity;
use open_tab_entities::schema;

use sea_orm::QueryOrder;
use sea_orm::prelude::Uuid;
use std::path::Display;
use std::{collections::HashMap, error::Error};

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;



use itertools::Itertools;

use crate::LoadedView;
use crate::TournamentParticipantsInfo;

pub struct LoadedRoundResultsView {
    pub view: RoundResultsView,
    pub round_id: Uuid
}

impl LoadedRoundResultsView {
    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<Self, anyhow::Error> where C: ConnectionTrait {
        Ok(
            LoadedRoundResultsView {
                round_id: round_uuid,
                view: RoundResultsView::load(db, round_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedRoundResultsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.tournament_debates.len() > 0 || changes.ballots.len() > 0 || changes.teams.len() > 0 || changes.participants.len() > 0 || changes.debate_backup_ballots.len() > 0 {
            println!("Refreshing round results view {}", self.round_id);
            self.view = RoundResultsView::load(db, self.round_id).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);
            println!("Done round results  view {}", self.round_id);

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
pub struct RoundResultsView {
    debates: Vec<ResultDebate>
}


impl RoundResultsView {
    async fn load<C>(db: &C, round_uuid: Uuid) -> Result<Self, anyhow::Error> where C: ConnectionTrait {

        Ok(RoundResultsView {
            debates: ResultDebate::load_all_from_round(db, round_uuid).await?
        })
    }
}