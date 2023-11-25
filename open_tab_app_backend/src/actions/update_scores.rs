

use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::debate_backup_ballot::DebateBackupBallot};

use sea_orm::prelude::*;

use open_tab_entities::derived_models::DisplayBallot;
use serde::{Serialize, Deserialize};
use open_tab_entities::domain::entity::LoadEntity;

use super::ActionTrait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScoresAction {
    pub debate_id: Uuid,
    pub update: ScoreUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoreUpdate {
    SetBallot(Uuid),
    NewBallot(DisplayBallot),
}



#[async_trait]
impl ActionTrait for UpdateScoresAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut groups = EntityGroup::new();
        let mut debate = open_tab_entities::domain::debate::TournamentDebate::get(db, self.debate_id).await?;
        match self.update {
            ScoreUpdate::SetBallot(uuid) => {
                debate.ballot_id = uuid;
                groups.add(Entity::TournamentDebate(debate));
            },
            ScoreUpdate::NewBallot(display_ballot) => {
                let mut ballot : Ballot = display_ballot.into();
                ballot.uuid = Uuid::new_v4();
                debate.ballot_id = ballot.uuid;
                let backup_ballot = DebateBackupBallot {
                    uuid: Uuid::new_v4(),
                    debate_id: self.debate_id,
                    ballot_id: ballot.uuid,
                    timestamp: chrono::offset::Local::now().naive_local(),
                };
                groups.add(Entity::Ballot(ballot));
                groups.add(Entity::TournamentDebate(debate));
                groups.add(Entity::DebateBackupBallot(backup_ballot));
            },
        }

        Ok(groups)
    }
}