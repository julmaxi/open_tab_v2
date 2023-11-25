use std::fmt::Display;


use std::{collections::HashMap, error::Error};

use async_trait::async_trait;



use sea_orm::prelude::*;
use open_tab_entities::{prelude::*};







use super::base::LoadedView;

pub use open_tab_entities::tab::{
    TabView,
    TeamRoundRole,
    TeamTabEntry,
    SpeakerTabEntry
};

pub struct LoadedTabView {
    pub view: TabView,
    pub tournament_uuid: Uuid
}

impl LoadedTabView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<LoadedTabView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            LoadedTabView {
                tournament_uuid,
                view: TabView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedTabView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.ballots.len() > 0 {
            self.view = TabView::load_from_tournament(db, self.tournament_uuid).await?;

            let mut out = HashMap::new();
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


#[derive(Debug)]
enum DrawViewError {
}

impl Display for DrawViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for DrawViewError {
}