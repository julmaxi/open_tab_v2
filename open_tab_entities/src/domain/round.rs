
use async_trait::async_trait;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

use crate::schema;


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum DrawType {
    StandardPreliminaryDraw,
    KnockoutDraw,
    PowerPaired,
    InversePowerPaired,
    BalancedPowerPaired,
    Randomized,
    BalancedRandomized
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, SimpleEntity)]
#[module_path = "crate::schema::tournament_round"]
#[tournament_id = "tournament_id"]
pub struct TournamentRound {
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub index: u64,
    #[serialize]
    pub draw_type: Option<DrawType>
}

impl TournamentRound {
    pub fn new(tournament_id: Uuid, index: u64) -> Self {
        TournamentRound {
            uuid: Uuid::new_v4(),
            tournament_id,
            index,
            draw_type: None
        }
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<TournamentRound>, DbErr> where C: ConnectionTrait {
        let rounds = schema::tournament_round::Entity::find().filter(schema::tournament_round::Column::TournamentId.eq(tournament_id)).all(db).await?;
        rounds.into_iter().map(|round| {
            Ok(TournamentRound {
                uuid: round.uuid,
                tournament_id: round.tournament_id,
                index: round.index as u64,
                draw_type: round.draw_type.map(|r| serde_json::from_str(&r).ok()).flatten()
            })
        }).collect()
    }
}
