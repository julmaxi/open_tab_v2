
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use open_tab_entities::{domain::entity::LoadEntity, EntityTypeId};
use open_tab_entities::schema::tournament_remote;


use sea_orm::prelude::Uuid;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::prelude::*;
use open_tab_entities::prelude::*;





use crate::LoadedView;


pub struct LoadedTournamentStatusView {
    pub view: TournamentStatusView,
    pub tournament_uuid: Uuid,
}

impl LoadedTournamentStatusView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
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
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_type(EntityTypeId::Tournament) {
            self.view = TournamentStatusView::load(db, self.tournament_uuid).await?;
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
pub struct TournamentStatusView {
    name: String,
    annoucements_password: Option<String>,
    remote_url: Option<String>,
    allow_self_declared_clashes: bool,
}

impl TournamentStatusView {
    pub async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let tournament = Tournament::get(db, tournament_id).await?;

        let remote = tournament_remote::Entity::find().filter(
            tournament_remote::Column::TournamentId.eq(tournament_id)
        )
            .one(db)
            .await?;

        Ok(Self {
            name: tournament.name,
            annoucements_password: tournament.annoucements_password,
            remote_url: remote.map(|r| r.url),
            allow_self_declared_clashes: tournament.allow_self_declared_clashes,
        })
    }
}