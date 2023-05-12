use std::{error::Error, fmt::{Display, Formatter}, collections::HashMap};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::debate_backup_ballot::DebateBackupBallot};

use sea_orm::prelude::*;

use crate::{round_results_view::DisplayBallot};
use serde::{Serialize, Deserialize};

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
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroups, Box<dyn Error>> where C: ConnectionTrait {
        let mut groups = EntityGroups::new();
        let mut debate = open_tab_entities::domain::debate::TournamentDebate::get_one(db, self.debate_id).await?;
        match self.update {
            ScoreUpdate::SetBallot(uuid) => {
                debate.current_ballot_uuid = uuid;
                groups.add(Entity::TournamentDebate(debate));
            },
            ScoreUpdate::NewBallot(display_ballot) => {
                let mut ballot : Ballot = display_ballot.into();
                ballot.uuid = Uuid::new_v4();
                debate.current_ballot_uuid = ballot.uuid;
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