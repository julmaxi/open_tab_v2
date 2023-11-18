pub mod draw_view;
pub mod tab_view;
pub mod rounds_view;
pub mod participants_list_view;
pub mod round_results_view;
pub mod tournament_tree_view;
pub mod round_publication_view;
pub mod feedback_views;
pub mod institutions_view;
pub mod venue_list_view;
pub mod tournament_status_view;
pub mod break_relevant_tab_view;
mod base;

pub use self::base::{LoadedView, TournamentParticipantsInfo};
use self::feedback_views::LoadedFeedbackDetailView;
use self::feedback_views::LoadedFeedbackOverviewView;
use self::round_publication_view::LoadedRoundPublicationView;
use self::rounds_view::LoadedRoundsView;
use self::participants_list_view::LoadedParticipantsListView;
use self::round_results_view::LoadedRoundResultsView;
use self::tab_view::LoadedTabView;
use self::tournament_tree_view::LoadedTournamentTreeView;
use self::institutions_view::LoadedInstitutionsView;
use self::venue_list_view::LoadedVenueListView;
use self::tournament_status_view::LoadedTournamentStatusView;
use self::break_relevant_tab_view::LoadedBreakRelevantTabView;


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
    RoundResults{round_uuid: Uuid},
    RoundPublication{round_uuid: Uuid},
    Tab{tournament_uuid: Uuid},
    TournamentTree{tournament_uuid: Uuid},
    FeedbackOverview{tournament_uuid: Uuid},
    FeedbackDetail{participant_id: Uuid},
    Institutions{tournament_uuid: Uuid},
    Venues{tournament_uuid: Uuid},
    TournamentStatus{tournament_uuid: Uuid},
    BreakRelevantTab{node_uuid: Uuid},
}

impl View {
    pub async fn load_json<C>(&self, db: &C) -> Result<String, anyhow::Error> where C: ConnectionTrait {
        let view = self.load(db).await?;
        view.view_string().await
    }

    pub async fn load<C>(&self, db: &C) -> Result<Box<dyn LoadedView>, anyhow::Error> where C: ConnectionTrait {
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
            },
            View::RoundPublication { round_uuid } => {
                Box::new(LoadedRoundPublicationView::load(db, *round_uuid).await?)
            },
            View::Tab { tournament_uuid } => {
                Box::new(LoadedTabView::load(db, *tournament_uuid).await?)
            },
            View::TournamentTree { tournament_uuid } => {
                Box::new(LoadedTournamentTreeView::load(db, *tournament_uuid).await?)
            },
            View::FeedbackOverview { tournament_uuid } => {
                Box::new(LoadedFeedbackOverviewView::load(db, *tournament_uuid).await?)
            },
            View::FeedbackDetail { participant_id } => {
                Box::new(LoadedFeedbackDetailView::load(db, *participant_id).await?)
            },
            View::Institutions { tournament_uuid } => {
                Box::new(LoadedInstitutionsView::load(db, *tournament_uuid).await?)
            },
            View::Venues { tournament_uuid } => {
                Box::new(LoadedVenueListView::load(db, *tournament_uuid).await?)
            },
            View::TournamentStatus { tournament_uuid } => {
                Box::new(LoadedTournamentStatusView::load(db, *tournament_uuid).await?)
            },
            View::BreakRelevantTab { node_uuid } => {
                Box::new(LoadedBreakRelevantTabView::load(db, *node_uuid).await?)
            },
        })
    }
}