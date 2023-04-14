pub mod draw_view;
pub mod tab_view;
pub mod rounds_view;
pub mod participants_list_view;
pub mod round_results_view;
mod base;

pub use self::base::{LoadedView, TournamentParticipantsInfo};
use self::rounds_view::LoadedRoundsView;
use self::participants_list_view::LoadedParticipantsListView;
use self::round_results_view::LoadedRoundResultsView;

use std::error::Error;

use self::draw_view::LoadedDrawView;

use sea_orm::ConnectionTrait;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum View {
    Draw{uuid: Uuid},
    RoundsOverview{tournament_uuid: Uuid},
    ParticipantsList{tournament_uuid: Uuid},
    RoundResults{round_uuid: Uuid}
}

impl View {
    pub async fn load_json<C>(&self, db: &C) -> Result<String, Box<dyn Error>> where C: ConnectionTrait {
        let view = self.load(db).await?;
        view.view_string().await
    }

    pub async fn load<C>(&self, db: &C) -> Result<Box<dyn LoadedView>, Box<dyn Error>> where C: ConnectionTrait {
        Ok(match self {
            View::Draw{uuid} => {
                Box::new(LoadedDrawView::load(db, *uuid).await?)
            }
            View::RoundsOverview { tournament_uuid } => {
                Box::new(LoadedRoundsView::load(db, *tournament_uuid).await?)
            }
            View::ParticipantsList { tournament_uuid } => {
                Box::new(LoadedParticipantsListView::load(db, *tournament_uuid).await?)
            },
            View::RoundResults { round_uuid } => {
                Box::new(LoadedRoundResultsView::load(db, *round_uuid).await?)
            }
        })
    }
}