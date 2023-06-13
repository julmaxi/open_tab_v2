use std::fmt::Display;
use std::hash::Hash;
use std::iter::{zip, self};
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use open_tab_entities::domain::entity::LoadEntity;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::{prelude::*, domain};

use open_tab_entities::schema::{self};

use itertools::Itertools;

use ordered_float::OrderedFloat;

use super::base::LoadedView;


pub struct LoadedRoundPublicationView {
    pub round: TournamentRound,
}

impl LoadedRoundPublicationView {
    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<LoadedRoundPublicationView, Box<dyn Error>> where C: ConnectionTrait {
        Ok(
            LoadedRoundPublicationView {
                round: TournamentRound::get(db, round_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedRoundPublicationView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.tournament_rounds.len() > 0 {
            self.round = TournamentRound::get(db, self.round.uuid).await?;
            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.round)?);
            Ok(Some(out))
        }
        else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.round)?)
    }
}
    