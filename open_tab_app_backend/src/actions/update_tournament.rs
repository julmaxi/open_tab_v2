

use crate::{venue_list_view::VenueOverview, ActionTrait};

use async_trait::async_trait;
use open_tab_entities::{domain::{entity::LoadEntity, tournament_venue::TournamentVenue}, prelude::Tournament, Entity, EntityGroup};
use sea_orm::prelude::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTournamentAction {
    tournament_id: Uuid,
    allow_self_declared_clashes: bool,
}

#[async_trait]
impl ActionTrait for UpdateTournamentAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new(
            self.tournament_id
        );

        let tournament = Tournament::get(db, self.tournament_id).await?;

        g.add(
            Entity::Tournament(
                Tournament {
                    uuid: self.tournament_id,
                    allow_self_declared_clashes: self.allow_self_declared_clashes,
                    ..tournament
                }
            )
        );

        Ok(
            g
        )
    }
}