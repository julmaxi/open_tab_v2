use sea_orm::prelude::Uuid;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::{prelude::*, QueryOrder};
use open_tab_entities::{schema::tournament_break_category, prelude::*, EntityTypeId};

use crate::LoadedView;

pub struct LoadedBreakCategoriesView {
    pub view: BreakCategoriesView,
    pub tournament_id: Uuid,
}

impl LoadedBreakCategoriesView {
    pub async fn load<C>(db: &C, tournament_uuid: Uuid) -> Result<Self, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        Ok(Self {
            tournament_id: tournament_uuid,
            view: BreakCategoriesView::load_from_tournament(db, tournament_uuid).await?,
        })
    }
}

#[async_trait]
impl LoadedView for LoadedBreakCategoriesView {
    async fn update_and_get_changes(
        &mut self,
        db: &sea_orm::DatabaseTransaction,
        changes: &EntityGroup,
    ) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_type(EntityTypeId::TournamentBreakCategory) {
            self.view = BreakCategoriesView::load_from_tournament(db, self.tournament_id).await?;

            let mut out = HashMap::new();
            out.insert(".".to_string(), serde_json::to_value(&self.view)?);

            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    async fn view_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self.view)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakCategoriesView {
    categories: Vec<BreakCategoryOverview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakCategoryOverview {
    uuid: Uuid,
    name: String,
}

impl BreakCategoriesView {
    async fn load_from_tournament<C>(
        db: &C,
        tournament_uuid: Uuid,
    ) -> Result<BreakCategoriesView, anyhow::Error>
    where
        C: sea_orm::ConnectionTrait,
    {
        let categories = tournament_break_category::Entity::find()
            .filter(tournament_break_category::Column::TournamentId.eq(tournament_uuid))
            .order_by_asc(tournament_break_category::Column::Name)
            .all(db)
            .await?;

        let category_overviews = categories
            .into_iter()
            .map(|category| BreakCategoryOverview {
                uuid: category.uuid,
                name: category.name,
            })
            .collect();

        Ok(BreakCategoriesView {
            categories: category_overviews,
        })
    }
}
