use std::{error::Error};



use open_tab_entities::{prelude::*};

use sea_orm::prelude::*;


use serde::{Serialize, Deserialize};

mod base;
mod update_draw;
mod update_participant;
mod upload_participants_list;
mod update_scores;
mod edit_tree;
mod generate_draw;
mod make_break;
mod update_round;
mod create_institution;
mod update_venues;

pub use self::base::ActionTrait;
pub use self::update_draw::UpdateDrawAction;
pub use self::update_participant::UpdateParticipantsAction;
pub use self::upload_participants_list::UploadParticipantsListAction;
pub use self::update_scores::UpdateScoresAction;
pub use self::edit_tree::EditTreeAction;
pub use self::generate_draw::ExecutePlanNodeAction;
pub use self::make_break::MakeBreakAction;
pub use self::update_round::UpdateRoundAction;
pub use self::create_institution::CreateInstitutionAction;
pub use self::update_venues::UpdateVenuesAction;
pub(crate) use self::edit_tree::EditTreeActionType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    UpdateDraw { action: UpdateDrawAction },
    UpdateParticipants { action: UpdateParticipantsAction },
    UpdateScores { action: UpdateScoresAction },
    UploadParticipantsList { action: UploadParticipantsListAction },
    EditTournamentTree { action: EditTreeAction },
    ExecutePlanNode { action: ExecutePlanNodeAction },
    MakeBreak { action: MakeBreakAction },
    UpdateRound { action: UpdateRoundAction },
    CreateInstitution { action: CreateInstitutionAction },
    UpdateVenues { action: UpdateVenuesAction },
}

impl Action {
    pub async fn execute<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: ConnectionTrait {
        match self {
            Action::UpdateDraw{action} => action.get_changes(db).await,
            Action::UpdateParticipants{action} => action.get_changes(db).await,
            Action::UpdateScores{action} => action.get_changes(db).await,
            Action::UploadParticipantsList { action } => action.get_changes(db).await,
            Action::EditTournamentTree { action } => action.get_changes(db).await,
            Action::ExecutePlanNode { action } => action.get_changes(db).await,
            Action::MakeBreak { action } => action.get_changes(db).await,
            Action::UpdateRound { action } => action.get_changes(db).await,
            Action::CreateInstitution { action } => action.get_changes(db).await,
            Action::UpdateVenues { action } => action.get_changes(db).await,
        }
    }
}
