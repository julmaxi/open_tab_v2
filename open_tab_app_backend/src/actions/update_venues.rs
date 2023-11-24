

use crate::{venue_list_view::VenueOverview, ActionTrait};

use migration::async_trait::async_trait;
use open_tab_entities::{EntityGroup, domain::tournament_venue::TournamentVenue, Entity, EntityGroupTrait};
use sea_orm::{ConnectionTrait, prelude::Uuid};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVenuesAction {
    #[serde(default)]
    updated_venues: Vec<VenueOverview>,
    #[serde(default)]
    added_venues: Vec<VenueOverview>,
    tournament_id: Uuid
}

#[async_trait]
impl ActionTrait for UpdateVenuesAction {
    async fn get_changes<C>(self, _db: &C) -> Result<EntityGroup, anyhow::Error> where C: ConnectionTrait {
        let mut g = EntityGroup::new();

        for venue in self.updated_venues.into_iter().chain(self.added_venues.into_iter().map(
            |v| VenueOverview {
                uuid: Uuid::new_v4(),
                ..v
            }
        )) {
            g.add(
                Entity::TournamentVenue(
                    TournamentVenue {
                        uuid: venue.uuid,
                        name: venue.name,
                        ordering_index: venue.ordering_index,
                        tournament_id: self.tournament_id
                    }
                )
            );
        }

        Ok(
            g
        )
    }
}