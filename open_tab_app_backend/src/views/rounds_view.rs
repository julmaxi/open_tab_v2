use sea_orm::prelude::Uuid;
use std::fmt::Display;
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;

use open_tab_entities::schema::{self, tournament_round};

use itertools::izip;
use itertools::Itertools;

use crate::LoadedView;

pub struct LoadedRoundsView {
    pub view: RoundsView,
    pub tournament_id: Uuid
    //TODO: Use this to cache team and participant names
    //to avoid a full reload every time
    //Alternatively, it would be interesting to try to implement
    //dependent views.
}

impl LoadedRoundsView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<LoadedRoundsView, Box<dyn Error>> where C: ConnectionTrait {
        Ok(
            LoadedRoundsView {
                tournament_id: tournament_uuid,
                view: RoundsView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedRoundsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroups) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>> {
        if changes.rounds.len() > 0 {
            self.view = RoundsView::load_from_tournament(db, self.tournament_id).await?;

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
pub struct RoundsView {
    rounds: Vec<RoundOverview>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundOverview {
    uuid: Uuid,
    round_number: i32,
    name: String,
}


impl RoundsView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<RoundsView, Box<dyn Error>> where C: ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(
            tournament_round::Column::TournamentId.eq(tournament_uuid)
        ).all(db).await?;

        let round_overviews = rounds.into_iter().map(|round| {
            RoundOverview {
                uuid: round.uuid,
                round_number: round.index,
                name: format!("Round {}", round.index),
            }
        }).collect();

        Ok(
            RoundsView {
                rounds: round_overviews
            }
        )
    }
}