use std::collections::HashMap;
use itertools::Itertools;
use sea_orm::prelude::*;

use open_tab_entities::{domain::participant_clash::ParticipantClash, prelude::{Participant, Ballot, SpeechRole}, domain::participant::ParticipantRole};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClashType {
    SpeakersHaveMetAsNonAligned { round: Uuid },
    SpeakersHaveMetAsTeamAndNonAligned { round: Uuid },
    SpeakersHaveMetAsTeam { round: Uuid },
    JudgeHasSeenSpeaker{ round: Uuid, judge_was_chair: bool, speaker_was_in_team: bool },
    JudgeHasSeenJudge{round: Uuid},
    DeclaredClash{severity: u16},
    InstitutionalClash{severity: u16, institution_id: Uuid},
    SameTeamClash
}