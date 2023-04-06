use std::{error::Error, fmt::{Display, Formatter}, collections::HashMap};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*};

use sea_orm::prelude::*;

use crate::draw_view::DrawBallot;
use serde::{Serialize, Deserialize};

mod base;
mod update_draw;
mod update_participant;
mod upload_participants_list;

pub use self::base::ActionTrait;
pub use self::update_draw::UpdateDrawAction;
pub use self::update_participant::UpdateParticipantsAction;
pub use self::upload_participants_list::UploadParticipantsListAction;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    UpdateDraw{action: UpdateDrawAction},
    UpdateParticipants{action: UpdateParticipantsAction}
}

impl Action {
    pub async fn execute<C>(self, db: &C) -> Result<EntityGroups, Box<dyn Error>> where C: ConnectionTrait {
        match self {
            Action::UpdateDraw{action} => action.get_changes(db).await,
            Action::UpdateParticipants{action} => action.get_changes(db).await
        }
    }
}