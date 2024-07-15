use open_tab_entities::{domain::tournament_venue, EntityTypeId};
use sea_orm::prelude::Uuid;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;




use crate::LoadedView;

pub struct LoadedVenueListView {
    pub view: VenueListView,
    pub tournament_id: Uuid
}

impl LoadedVenueListView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            Self {
                tournament_id: tournament_uuid,
                view: VenueListView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedVenueListView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_type(EntityTypeId::TournamentVenue) {
            self.view = VenueListView::load_from_tournament(db, self.tournament_id).await?;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueListView {
    venues: Vec<VenueOverview>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueOverview {
    pub uuid: Uuid,
    pub name: String,
    pub ordering_index: i32
}


impl VenueListView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<VenueListView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let venues = tournament_venue::TournamentVenue::get_all_in_tournament(db, tournament_uuid).await?;

        let venues = venues.into_iter().map(|venue| {
            VenueOverview {
                uuid: venue.uuid,
                name: venue.name,
                ordering_index: venue.ordering_index
            }
        }).collect();

        Ok(
            VenueListView {
                venues
            }
        )
    }
}