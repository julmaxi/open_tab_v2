
use async_trait::async_trait;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum TeamDrawMode {
    PowerPaired,
    InversePowerPaired,
    BalancedPowerPaired,
    Random,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum SpeakerDrawMode {
    PowerPaired,
    Random,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum TeamAssignmentRule {
    Random,
    Fixed,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TabDrawConfig {
    pub team_draw: TeamDrawMode,
    pub team_assignment_rule: TeamAssignmentRule,
    pub speaker_draw: SpeakerDrawMode
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum DrawType {
    Preliminary,
    KnockoutDraw,
    TabDraw {
        config: TabDrawConfig
    }
}


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum RoundState {
    NotStarted,
    InProgress,
    Finished,
}

impl Default for RoundState {
    fn default() -> Self {
        RoundState::NotStarted
    }
}

impl From<String> for RoundState {
    fn from(s: String) -> Self {
        // TODO: We need to find a general solution here
        let s = s.strip_prefix("\"").unwrap_or(&s).strip_suffix("\"").unwrap_or(&s);
        match s {
            "NotStarted" => RoundState::NotStarted,
            "InProgress" => RoundState::InProgress,
            "Finished" => RoundState::Finished,
            _ => panic!("Invalid round state {}", s)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize, Clone, SimpleEntity)]
#[module_path = "crate::schema::tournament_round"]
#[tournament_id = "tournament_id"]
pub struct TournamentRound {
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub index: u64,
    #[serialize]
    pub draw_type: Option<DrawType>,
    pub motion: Option<String>,
    pub info_slide: Option<String>,
    pub is_silent: bool,

    pub draw_release_time: Option<chrono::NaiveDateTime>,
    pub team_motion_release_time: Option<chrono::NaiveDateTime>,
    pub debate_start_time: Option<chrono::NaiveDateTime>,
    pub full_motion_release_time: Option<chrono::NaiveDateTime>,
    pub round_close_time: Option<chrono::NaiveDateTime>,
}

impl TournamentRound {
    pub fn new(tournament_id: Uuid, index: u64) -> Self {
        TournamentRound {
            uuid: Uuid::new_v4(),
            tournament_id,
            index,
            draw_type: None,
            ..Default::default()
        }
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<TournamentRound>, DbErr> where C: ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(schema::tournament_round::Column::TournamentId.eq(tournament_id)).all(db).await?;
        Ok(rounds.into_iter().map(TournamentRound::from_model).collect())
    }
}
