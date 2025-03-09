



use open_tab_entities::prelude::*;

use serde::{Serialize, Deserialize};

mod base;
mod update_draw;
mod update_participant;
mod upload_participants_list;
mod update_scores;
mod edit_tree;
pub(crate) mod execute_plan_node;
mod update_round;
mod create_institution;
mod update_venues;
mod set_manual_break;
mod import_feedback_system;
mod set_adjudicator_break;
mod update_teams;
mod set_break_release;
mod redraw_round;
mod update_tournament;
mod update_clashes_action;
mod discard_ballot;
mod update_feedback_system;

pub use self::base::ActionTrait;
pub use self::update_draw::UpdateDrawAction;
pub use self::update_participant::UpdateParticipantsAction;
pub use self::upload_participants_list::UploadParticipantsListAction;
pub use self::update_scores::UpdateScoresAction;
pub use self::edit_tree::EditTreeAction;
pub use self::execute_plan_node::ExecutePlanNodeAction;
pub use self::set_manual_break::SetManualBreakAction;
pub use self::update_round::UpdateRoundAction;
pub use self::create_institution::CreateInstitutionAction;
pub use self::update_venues::UpdateVenuesAction;
pub use self::import_feedback_system::ImportFeedbackSystemAction;
pub use self::set_adjudicator_break::SetAdjudicatorBreakAction;
pub use self::update_teams::UpdateTeamsAction;
pub use self::set_break_release::SetBreakReleaseAction;
pub use self::redraw_round::RedrawRoundAction;
pub use self::update_tournament::UpdateTournamentAction;
pub use self::update_clashes_action::UpdateClashes;
pub use self::discard_ballot::DiscardBallotAction;
pub use self::update_feedback_system::UpdateFeedbackSystemAction;

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
    UpdateRound { action: UpdateRoundAction },
    CreateInstitution { action: CreateInstitutionAction },
    UpdateVenues { action: UpdateVenuesAction },
    SetManualBreak { action: SetManualBreakAction },
    ImportFeedbackSystem { action: ImportFeedbackSystemAction },
    SetAdjudicatorBreak { action: SetAdjudicatorBreakAction },
    UpdateTeams { action: UpdateTeamsAction },
    SetBreakRelease { action: SetBreakReleaseAction },
    RedrawRound { action: RedrawRoundAction },
    UpdateTournament { action: UpdateTournamentAction },
    UpdateClashes { action: UpdateClashes },
    DiscardBallot { action: DiscardBallotAction },
    UpdateFeedbackSystem { action: UpdateFeedbackSystemAction },
}

impl Action {
    pub async fn execute<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        match self {
            Action::UpdateDraw{action} => action.get_changes(db).await,
            Action::UpdateParticipants{action} => action.get_changes(db).await,
            Action::UpdateScores{action} => action.get_changes(db).await,
            Action::UploadParticipantsList { action } => action.get_changes(db).await,
            Action::EditTournamentTree { action } => action.get_changes(db).await,
            Action::ExecutePlanNode { action } => action.get_changes(db).await,
            Action::UpdateRound { action } => action.get_changes(db).await,
            Action::CreateInstitution { action } => action.get_changes(db).await,
            Action::UpdateVenues { action } => action.get_changes(db).await,
            Action::SetManualBreak { action } => action.get_changes(db).await,
            Action::ImportFeedbackSystem { action } => action.get_changes(db).await,
            Action::SetAdjudicatorBreak { action } => action.get_changes(db).await,
            Action::UpdateTeams { action } => action.get_changes(db).await,
            Action::SetBreakRelease { action } => action.get_changes(db).await,
            Action::RedrawRound { action } => action.get_changes(db).await,
            Action::UpdateTournament { action } => action.get_changes(db).await,
            Action::UpdateClashes { action } => action.get_changes(db).await,
            Action::DiscardBallot { action } => action.get_changes(db).await,
            Action::UpdateFeedbackSystem { action } => action.get_changes(db).await,
        }
    }
}
