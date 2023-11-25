


use async_trait::async_trait;
use itertools::{Itertools, izip};
use sea_orm::{prelude::*, ActiveValue, QueryOrder};
use serde::{Serialize, Deserialize};


use crate::schema;
use crate::utilities::BatchLoad;

use super::{TournamentEntity, utils};
use super::entity::LoadEntity;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum MinorDrawConfig {
    PowerPaired,
    InversePowerPaired,
    BalancedPowerPaired,
    Random
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum TeamFoldMethod {
    PowerPaired,
    InversePowerPaired,
    BalancedPowerPaired,
    Random
}


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(tag = "non_aligned_fold_method")]
pub enum NonAlignedFoldMethod {
    TabOrder,
    Random
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(tag = "team_assignment_rule")]
pub enum TeamAssignmentRule {
    Random,
    InvertPrevious,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct FoldDrawConfig {
    pub team_fold_method: TeamFoldMethod,
    #[serde(flatten)]
    pub team_assignment_rule: TeamAssignmentRule,
    #[serde(flatten)]
    pub non_aligned_fold_method: NonAlignedFoldMethod,
}

impl FoldDrawConfig {
    pub fn default_ko_fold() -> Self {
        FoldDrawConfig {
            team_fold_method: TeamFoldMethod::InversePowerPaired,
            team_assignment_rule: TeamAssignmentRule::Random,
            non_aligned_fold_method: NonAlignedFoldMethod::Random,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum RoundGroupConfig {
    Preliminaries { num_roundtrips: i32 },
    FoldDraw {
        round_configs: Vec<FoldDrawConfig>
    },
}

impl RoundGroupConfig {
    pub fn num_rounds(&self) -> i32 {
        match self {
            RoundGroupConfig::Preliminaries {num_roundtrips} => num_roundtrips * 3,
            RoundGroupConfig::FoldDraw {round_configs} => round_configs.len() as i32,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum BreakConfig {
    Manual,
    TabBreak {num_debates: u32},
    KnockoutBreak,
    TwoThirdsBreak,
    TimBreak,
}

impl BreakConfig {
    pub fn human_readable_description(&self) -> String {
        match self {
            BreakConfig::TabBreak{num_debates } => format!("Top {0} break", num_debates * 2),
            BreakConfig::TwoThirdsBreak => "Upper 2/3rds break".to_string(),
            BreakConfig::KnockoutBreak => "Debate winners break".to_string(),
            BreakConfig::TimBreak => "Upper 1/3rd breaks, along with non-aligned".to_string(),
            BreakConfig::Manual => "Manual Break".to_string()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum PlanNodeConfig {
    RoundGroup{config: RoundGroupConfig},
    Break{config: BreakConfig},
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum PlanNodeType {
    Round{config: RoundGroupConfig, rounds: Vec<Uuid>},
    Break{config: BreakConfig, break_id: Option<Uuid>}
}


impl PlanNodeType {
    pub fn new_break(config: BreakConfig) -> Self {
        PlanNodeType::Break{config, break_id: None}
    }
}

/*
impl BreakType {
    pub fn human_readable_description(&self) -> String {
        match self {
            BreakType::TabBreak{num_debates} => format!("Top {0} break", num_debates * 2),
            BreakType::TwoThirdsBreak => "Upper 2/3rds break".to_string(),
            BreakType::KOBreak => "Debate winners break".to_string(),
            BreakType::TimBreak => "Upper 1/3rd breaks, along with non-aligned".to_string(),
        }
    }
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum TournamentBreakSourceRoundType {
    Tab,
    Knockout,
}
*/


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TournamentPlanNodeRound {
    pub uuid: Uuid,
    pub position: i32,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TournamentPlanNode {
    pub uuid: Uuid,
    pub tournament_id: Uuid,
    pub config: PlanNodeType,
}

impl TournamentPlanNode {
    pub fn new(tournament_id: Uuid, config: PlanNodeType) -> Self {
        TournamentPlanNode {
            uuid: Uuid::new_v4(),
            tournament_id,
            config,
        }
    }

    pub fn from_rows(
        node_row: schema::tournament_plan_node::Model,
        rounds: Vec<schema::tournament_plan_node_round::Model>,
    ) -> Result<Self, anyhow::Error> {
        let config = serde_json::from_str::<PlanNodeConfig>(&node_row.config)?;
        match config {
            PlanNodeConfig::RoundGroup { config } => {
                Ok(Self {
                    uuid: node_row.uuid,
                    tournament_id: node_row.tournament_id,
                    config: PlanNodeType::Round{config, rounds: rounds.into_iter().map(|r| r.round_id).collect()},
                })
            },
            PlanNodeConfig::Break { config } => {
                Ok(Self {
                    uuid: node_row.uuid,
                    tournament_id: node_row.tournament_id,
                    config: PlanNodeType::Break{config, break_id: node_row.break_id},
                })
            }
        }
    }

    pub async fn get_all_in_tournament<C>(db: &C, tournament_id: Uuid) -> Result<Vec<Self>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let nodes = schema::tournament_plan_node::Entity::find()
            .filter(schema::tournament_plan_node::Column::TournamentId.eq(tournament_id))
            .all(db)
            .await?;

        let rounds = nodes.load_many(schema::tournament_plan_node_round::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            nodes,
            rounds,
        ).into_iter().map(|(node, rounds)| {
            Self::from_rows(node, rounds)
        }).collect();
        r
    }
}


#[async_trait]
impl LoadEntity for TournamentPlanNode {
    async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Self>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let nodes = schema::tournament_plan_node::Entity::batch_load(db, uuids).await?;
        let exists_mask = nodes.iter().map(|b| b.is_some()).collect::<Vec<_>>();

        let nodes = nodes.into_iter().flatten().collect::<Vec<_>>();

        let rounds = nodes.load_many(schema::tournament_plan_node_round::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            nodes,
            rounds,
        ).into_iter().map(|(node, rounds)| {
            Self::from_rows(node, rounds)
        }).collect();
        r.map(|r| utils::pad(r, &exists_mask))
    }
}

#[async_trait]
impl TournamentEntity for TournamentPlanNode {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        let empty_vec = vec![];
        let (model, rounds) = match &self.config {
            PlanNodeType::Break { config, break_id } => {
                (schema::tournament_plan_node::ActiveModel {
                    uuid: ActiveValue::Set(self.uuid),
                    tournament_id: ActiveValue::Set(self.tournament_id),
                    break_id: ActiveValue::Set(break_id.clone()),
                    config: ActiveValue::Set(serde_json::to_string(
                        &PlanNodeConfig::Break { config: config.clone() })?
                    ),
                }, &empty_vec)
            },
            PlanNodeType::Round { config, rounds } => {
                (schema::tournament_plan_node::ActiveModel {
                    uuid: ActiveValue::Set(self.uuid),
                    tournament_id: ActiveValue::Set(self.tournament_id),
                    break_id: ActiveValue::Set(None),
                    config: ActiveValue::Set(serde_json::to_string(
                        &PlanNodeConfig::RoundGroup { config: config.clone() })?
                    ),
                }, rounds)
            }
        };

        if guarantee_insert {
            model.insert(db).await?;
        }
        else {
            let prev_model = schema::tournament_plan_node::Entity::find_by_id(self.uuid).one(db).await?;

            if let Some(_) = prev_model {
                model.update(db).await?;
            } else {
                model.insert(db).await?;
            }
        }

        let num_required_rounds = rounds.len();
        if guarantee_insert {
            if num_required_rounds > 0 {
                schema::tournament_plan_node_round::Entity::insert_many((0..num_required_rounds).map(|i| {
                    schema::tournament_plan_node_round::ActiveModel {
                        plan_node_id: ActiveValue::Set(self.uuid),
                        round_id: ActiveValue::Set(rounds[i]),
                        position: ActiveValue::Set(i as i32),
                    }
                }).collect_vec()).exec(db).await?;    
            }
        } else {
            let prev_rounds = schema::tournament_plan_node_round::Entity::find()
                .filter(schema::tournament_plan_node_round::Column::PlanNodeId.eq(self.uuid))
                .order_by_asc(schema::tournament_plan_node_round::Column::Position)
                .all(db)
                .await?;

            let rounds_to_keep = prev_rounds.iter().take(num_required_rounds).collect_vec();

            for (i, round_) in rounds_to_keep.iter().enumerate() {
                let model = schema::tournament_plan_node_round::ActiveModel {
                    plan_node_id: ActiveValue::Set(self.uuid),
                    round_id: ActiveValue::Set(rounds[i]),
                    position: ActiveValue::Set(i as i32),
                };

                if round_.round_id != rounds[i] {
                    model.update(db).await?;
                }
            }

            if num_required_rounds < prev_rounds.len() {
                schema::tournament_plan_node_round::Entity::delete_many().filter(
                    schema::tournament_plan_node_round::Column::PlanNodeId.eq(self.uuid)
                        .and(schema::tournament_plan_node_round::Column::Position.gte(num_required_rounds as i32))
                ).exec(db).await?;
            }
            else if num_required_rounds > prev_rounds.len() {
                let to_insert: Vec<schema::tournament_plan_node_round::ActiveModel> = (prev_rounds.len()..num_required_rounds).map(|i| {
                    schema::tournament_plan_node_round::ActiveModel {
                        plan_node_id: ActiveValue::Set(self.uuid),
                        round_id: ActiveValue::Set(rounds[i]),
                        position: ActiveValue::Set(i as i32),
                    }
                }).collect_vec();

                schema::tournament_plan_node_round::Entity::insert_many(to_insert).exec(db).await?;
            }
        };

        Ok(())
    }

    async fn get_many_tournaments<C>(_db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        return Ok(entities.iter().map(|team| {
            Some(team.tournament_id)
        }).collect());
    }
    
    async fn delete_many<C>(db: &C, ids: Vec<Uuid>) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        schema::tournament_break::Entity::delete_many().filter(schema::tournament_break::Column::Uuid.is_in(ids)).exec(db).await?;
        Ok(())
    }
}
