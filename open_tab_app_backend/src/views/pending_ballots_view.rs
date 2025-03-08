use sea_orm::prelude::Uuid;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use sea_orm::{prelude::*, QueryOrder, QuerySelect};
use open_tab_entities::{prelude::*, EntityTypeId};

use open_tab_entities::schema::{debate_backup_ballot, tournament, tournament_debate, tournament_round};




use crate::LoadedView;

pub struct LoadedPendingBallotsView {
    pub view: PendingBallotsView,
    pub tournament_id: Uuid
}

impl LoadedPendingBallotsView {
    pub async fn load<C>(db: &C, tournament_id: Uuid) -> Result<Self, anyhow::Error> where C: sea_orm::ConnectionTrait {
        Ok(
            Self {
                tournament_id: tournament_id,
                view: PendingBallotsView::load_from_tournament(db, tournament_id).await?,
            }
        )
    }
}

#[async_trait]
impl LoadedView for LoadedPendingBallotsView {
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error> {
        if changes.has_changes_for_type(EntityTypeId::DebateBackupBallot) {
            self.view = PendingBallotsView::load_from_tournament(db, self.tournament_id).await?;

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
pub struct PendingBallotsView {
    pending_ballot_counts: HashMap<Uuid, i32>
}


impl PendingBallotsView {
    async fn load_from_tournament<C>(db: &C, tournament_id: Uuid) -> Result<PendingBallotsView, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let num_pending_ballots = debate_backup_ballot::Entity::find()
            .select_only()
            .column_as(tournament_round::Column::Uuid, "uuid")
            .column_as(tournament_round::Column::Uuid.count(), "count")
            .inner_join(tournament_debate::Entity)
            .join(sea_orm::JoinType::InnerJoin, tournament_debate::Relation::TournamentRound.def())
            .filter(tournament_round::Column::TournamentId.eq(tournament_id))
            .filter(debate_backup_ballot::Column::WasSeen.eq(false))
            .group_by(tournament_debate::Column::Uuid)
            .into_tuple::<(Uuid, i32)>()
            .all(db)
            .await?;

        Ok(
            PendingBallotsView {
                pending_ballot_counts: num_pending_ballots.into_iter().collect()
            }
        )
    }
}