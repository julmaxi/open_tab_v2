use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::entity::LoadEntity};

use sea_orm::prelude::*;


use serde::{Serialize, Deserialize};

use open_tab_server::patch::PatchValue;

use super::ActionTrait;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundUpdate {
    #[serde(default)]
    pub motion: PatchValue<Option<String>>,
    #[serde(default)]
    pub info_slide: PatchValue<Option<String>>,
    #[serde(default)]
    pub is_silent: PatchValue<bool>,

    #[serde(default)]
    pub draw_release_time: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    pub team_motion_release_time: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    pub debate_start_time: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    pub full_motion_release_time: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    pub round_close_time: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    pub feedback_release_time: PatchValue<Option<chrono::NaiveDateTime>>,
    #[serde(default)]
    pub silent_round_results_release_time: PatchValue<Option<chrono::NaiveDateTime>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoundAction {
    update: RoundUpdate,
    round_id: Uuid
}


#[async_trait]
impl ActionTrait for UpdateRoundAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let mut existing_round = open_tab_entities::domain::round::TournamentRound::get(db, self.round_id).await?;
        let mut groups = EntityGroup::new(
            existing_round.tournament_id
        );

        if let PatchValue::Set(motion) = self.update.motion {
            existing_round.motion = motion;
        }
        if let PatchValue::Set(info_slide) = self.update.info_slide {
            existing_round.info_slide = info_slide;
        }
        if let PatchValue::Set(is_silent) = self.update.is_silent {
            existing_round.is_silent = is_silent; 
        }
        if let PatchValue::Set(draw_release_time) = self.update.draw_release_time {
            existing_round.draw_release_time = draw_release_time;
        }
        if let PatchValue::Set(team_motion_release_time) = self.update.team_motion_release_time {
            existing_round.team_motion_release_time = team_motion_release_time;
        }
        if let PatchValue::Set(debate_start_time) = self.update.debate_start_time {
            existing_round.debate_start_time = debate_start_time;
        }
        if let PatchValue::Set(full_motion_release_time) = self.update.full_motion_release_time {
            existing_round.full_motion_release_time = full_motion_release_time;
        }
        if let PatchValue::Set(round_close_time) = self.update.round_close_time {
            existing_round.round_close_time = round_close_time;
        }
        if let PatchValue::Set(feedback_release_time) = self.update.feedback_release_time {
            existing_round.feedback_release_time = feedback_release_time;
        }
        if let PatchValue::Set(silent_round_results_release_time) = self.update.silent_round_results_release_time {
            existing_round.silent_round_results_release_time = silent_round_results_release_time;
        }

        groups.add(Entity::TournamentRound(existing_round));

        Ok(groups)
    }
}