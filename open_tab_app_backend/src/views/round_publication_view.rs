


use std::collections::HashMap;

use async_trait::async_trait;
use open_tab_entities::domain::entity::LoadEntity;


use sea_orm::prelude::*;
use open_tab_entities::prelude::*;







use super::base::LoadedView;


pub struct LoadedRoundPublicationView {
    pub round: TournamentRound,
}

impl LoadedRoundPublicationView {
    pub async fn load<C>(db: &C, round_uuid: Uuid) -> Result<LoadedRoundPublicationView, anyhow::Error> where C: ConnectionTrait {
        Ok(
            LoadedRoundPublicationView {
                round: TournamentRound::get(db, round_uuid).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedRoundPublicationView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
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

    async fn view_string(&self) -> Result<String, anyhow::Error> {
        Ok(serde_json::to_string(&self.round)?)
    }
}
    