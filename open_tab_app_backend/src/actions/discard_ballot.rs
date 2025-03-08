

use open_tab_entities::{domain::entity::LoadEntity, Entity, EntityGroup};
use sea_orm::prelude::Uuid;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::actions::ActionTrait;

use open_tab_entities::domain::debate_backup_ballot::DebateBackupBallot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscardBallotAction {
    pub tournament_id: Uuid,
    pub backup_ballot_id: Uuid
}


#[async_trait]
impl ActionTrait for DiscardBallotAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut g = EntityGroup::new(self.tournament_id);
        let mut backup_ballot = DebateBackupBallot::get(db, self.backup_ballot_id).await?;

        backup_ballot.was_seen = true;

        g.add(Entity::DebateBackupBallot(backup_ballot));
        Ok(
            g
        )
    }
}