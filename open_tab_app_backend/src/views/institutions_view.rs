use sea_orm::prelude::Uuid;

use std::{collections::HashMap};

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::{prelude::*, QueryOrder};
use open_tab_entities::prelude::*;

use open_tab_entities::schema::{tournament_institution};




use crate::LoadedView;

pub struct LoadedInstitutionsView {
    pub view: InstitutionsView,
    pub tournament_id: Uuid
}

impl LoadedInstitutionsView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error> where C: ConnectionTrait {
        Ok(
            Self {
                tournament_id: tournament_uuid,
                view: InstitutionsView::load_from_tournament(db, tournament_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedInstitutionsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.tournament_institutions.len() > 0 {
            self.view = InstitutionsView::load_from_tournament(db, self.tournament_id).await?;

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
pub struct InstitutionsView {
    institutions: Vec<InstitutionOverview>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstitutionOverview {
    uuid: Uuid,
    name: String,
}


impl InstitutionsView {
    async fn load_from_tournament<C>(db: &C, tournament_uuid: Uuid) -> Result<InstitutionsView, anyhow::Error> where C: ConnectionTrait {
        let rounds = tournament_institution::Entity::find().filter(
            tournament_institution::Column::TournamentId.eq(tournament_uuid)
        ).order_by_asc(tournament_institution::Column::Name).all(db).await?;

        let round_overviews = rounds.into_iter().map(|institution| {
            InstitutionOverview {
                uuid: institution.uuid,
                name: institution.name
            }
        }).collect();

        Ok(
            InstitutionsView {
                institutions: round_overviews
            }
        )
    }
}