use std::collections::HashMap;
use std::hash::Hash;

use async_trait::async_trait;
use itertools::{Itertools, izip};
use sea_orm::{prelude::*, ActiveValue, QueryOrder};
use serde::{Serialize, Deserialize};


use crate::schema;
use crate::utilities::BatchLoad;

use super::{BoundTournamentEntityTrait, utils};
use super::entity::{LoadEntity, TournamentEntityTrait};

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
    Random,
    HalfRandom // For silly Regelkommission variant
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
    TabBreak { num_teams: u32, num_non_aligned: u32 },
    KnockoutBreak,
    TwoThirdsBreak,
    TimBreak,
    TeamOnlyKnockoutBreak,
    BestSpeakerOnlyBreak,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TournamentEligibleBreakCategory {
    pub category_id: Uuid,
    pub config: EligibilityConfig,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct EligibilityConfig {
    pub team_eligibility_mode: TeamEligibilityMode,
    pub non_aligned_eligibility_mode: NonAlignedEligibilityMode,
    pub adjudicator_eligibility_mode: AdjudicatorEligibilityMode,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum TeamEligibilityMode {
    DoNotRestrict,
    AnyEligible,
    MajorityEligible,
    AllEligible,    
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum NonAlignedEligibilityMode {
    DoNotRestrict,
    AllEligible,
    AllInEligibleTeams,
    AllEligibleInEligibleTeams,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum AdjudicatorEligibilityMode {
    DoNotRestrict,
    AllEligible,
}

impl BreakConfig {
    pub fn human_readable_description(&self) -> String {
        match self {
            BreakConfig::TabBreak{num_teams, num_non_aligned } => {
                if *num_teams == 0 {
                    return format!("Top {0} Speakers", num_non_aligned)
                }
                else if num_teams % 2 == 0 && num_non_aligned % 3 == 0 && num_non_aligned / 3 == num_teams / 2 {
                    return format!("Top {0} break", num_teams)
                }
                else {
                    return format!("Top {0} teams, {1} non-aligned", num_teams, num_non_aligned)
                }
            },
            BreakConfig::TwoThirdsBreak => "Upper 2/3rds".to_string(),
            BreakConfig::KnockoutBreak => "Debate winners".to_string(),
            BreakConfig::TimBreak => "Upper 1/3rd, along with non-aligned".to_string(),
            BreakConfig::Manual => "Manual".to_string(),
            BreakConfig::TeamOnlyKnockoutBreak => "Winning Team".to_string(),
            BreakConfig::BestSpeakerOnlyBreak => "Best Speaker".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum PlanNodeConfig {
    RoundGroup{config: RoundGroupConfig},
    Break { config: BreakConfig },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum PlanNodeType {
    Round {
config: RoundGroupConfig,
rounds: Vec<Uuid>,
},
    Break {
        config: BreakConfig,
        break_id: Option<Uuid>,
        eligible_categories: Vec<TournamentEligibleBreakCategory>,
        suggested_award_title: Option<String>,
        suggested_break_award_prestige: Option<i32>,
        max_breaking_adjudicator_count: Option<i32>,
        is_only_award: bool,
        suggested_award_series_key: Option<String>,
    },
}

impl PlanNodeType {
    pub fn new_break(config: BreakConfig) -> Self {
        PlanNodeType::Break {
            config,
            break_id: None,
            eligible_categories: vec![],
            suggested_award_title: None,
            max_breaking_adjudicator_count: None,
            is_only_award: false,
            suggested_break_award_prestige: None,
            suggested_award_series_key: None
        }
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
        break_categories: Vec<schema::tournament_break_eligible_category::Model>,
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
                    config: PlanNodeType::Break {
                        config,
                        is_only_award: node_row.is_only_award,
                        max_breaking_adjudicator_count: node_row.max_breaking_adjudicator_count,
                        suggested_award_title: node_row.suggested_award_title,
                        break_id: node_row.break_id,
                        suggested_break_award_prestige: node_row.suggested_award_prestige,
                        suggested_award_series_key: node_row.suggested_award_series_key,
                        eligible_categories: break_categories.into_iter().map(|r| {
                            Ok(TournamentEligibleBreakCategory {
                                category_id: r.tournament_break_category_id,
                                config: serde_json::from_value::<EligibilityConfig>(r.config)?,
                            })
                        },
                    ).collect::<Result<Vec<_>, anyhow::Error>>()?
                    }
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

        let break_categories = nodes.load_many(schema::tournament_break_eligible_category::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            nodes,
            rounds,
            break_categories
        ).into_iter().map(|(node, rounds, categories)| {
            Self::from_rows(node, rounds, categories)
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
        let break_categories = nodes.load_many(schema::tournament_break_eligible_category::Entity, db).await?;

        let r : Result<Vec<_>, _> = izip!(
            nodes,
            rounds,
            break_categories
        ).into_iter().map(|(node, rounds, categories)| {
            Self::from_rows(node, rounds, categories)
        }).collect();
        r.map(|r| utils::pad(r, &exists_mask))
    }
}

#[async_trait]
impl<C> BoundTournamentEntityTrait<C> for TournamentPlanNode where C: ConnectionTrait {
    async fn save(&self, db: &C, guarantee_insert: bool) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        let empty_vec = vec![];
        let (model, rounds) = match &self.config {
            PlanNodeType::Break { config, break_id, suggested_award_title, max_breaking_adjudicator_count, is_only_award, suggested_break_award_prestige, suggested_award_series_key, .. } => {
                (schema::tournament_plan_node::ActiveModel {
                    uuid: ActiveValue::Set(self.uuid),
                    tournament_id: ActiveValue::Set(self.tournament_id),
                    break_id: ActiveValue::Set(break_id.clone()),
                    config: ActiveValue::Set(serde_json::to_string(
                        &PlanNodeConfig::Break { config: config.clone() })?
                    ),
                    suggested_award_title: ActiveValue::Set(suggested_award_title.clone()),
                    max_breaking_adjudicator_count: ActiveValue::Set(*max_breaking_adjudicator_count),
                    is_only_award: ActiveValue::Set(*is_only_award),
                    suggested_award_prestige: ActiveValue::Set(suggested_break_award_prestige.clone()),
                    suggested_award_series_key: ActiveValue::Set(suggested_award_series_key.clone()),
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
                    suggested_award_title: ActiveValue::Set(None),
                    max_breaking_adjudicator_count: ActiveValue::Set(None),
                    is_only_award: ActiveValue::Set(false),
                    suggested_award_prestige: ActiveValue::Set(None),
                    suggested_award_series_key: ActiveValue::Set(None),
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

        if let PlanNodeType::Break { eligible_categories, .. } = &self.config {
            let mut to_insert = vec![];
            let mut to_update = vec![];

            let previous_categories = schema::tournament_break_eligible_category::Entity::find()
                .filter(schema::tournament_break_eligible_category::Column::TournamentPlanNodeId.eq(self.uuid))
                .all(db)
                .await?;

            let prev_categories_by_id = previous_categories.iter().map(|c| (c.tournament_break_category_id, c)).collect::<HashMap<_, _>>();

            let to_delete = previous_categories.iter().filter(|c| {
                !eligible_categories.iter().any(|e| e.category_id == c.tournament_break_category_id)
            }).map(|c| c.tournament_break_category_id).collect_vec();

            for category in eligible_categories {
                let prev_category = prev_categories_by_id.get(&category.category_id);
                if let Some(_) = prev_category {
                    to_update.push(schema::tournament_break_eligible_category::ActiveModel {
                        tournament_plan_node_id: ActiveValue::Unchanged(self.uuid),
                        tournament_break_category_id: ActiveValue::Unchanged(category.category_id),
                        config: ActiveValue::Set(serde_json::to_value(&category.config)?),
                    });
                } else {
                    to_insert.push(schema::tournament_break_eligible_category::ActiveModel {
                        tournament_plan_node_id: ActiveValue::Set(self.uuid),
                        tournament_break_category_id: ActiveValue::Set(category.category_id),
                        config: ActiveValue::Set(serde_json::to_value(&category.config)?),
                    });
                }
            }

            if to_insert.len() > 0 {
                schema::tournament_break_eligible_category::Entity::insert_many(to_insert).exec(db).await?;
            }

            for update in to_update {
                update.update(db).await?;
            }

            if to_delete.len() > 0 {
                schema::tournament_break_eligible_category::Entity::delete_many()
                    .filter(schema::tournament_break_eligible_category::Column::TournamentPlanNodeId.eq(self.uuid)
                        .and(schema::tournament_break_eligible_category::Column::TournamentBreakCategoryId.is_in(to_delete)))
                    .exec(db).await?;
            }
        }

        Ok(())
    }

    async fn get_many_tournaments(_db: &C, entities: &Vec<&Self>) -> Result<Vec<Option<Uuid>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        return Ok(entities.iter().map(|team| {
            Some(team.tournament_id)
        }).collect());
    }
    
    async fn delete_many(db: &C, ids: Vec<Uuid>) -> Result<(), anyhow::Error> where C: sea_orm::ConnectionTrait {
        schema::tournament_plan_node::Entity::delete_many().filter(schema::tournament_plan_node::Column::Uuid.is_in(ids)).exec(db).await?;
        Ok(())
    }
}

impl TournamentEntityTrait for TournamentPlanNode {
    fn get_related_uuids(&self) -> Vec<Uuid> {
        let mut out = match &self.config {
            PlanNodeType::Round { rounds, .. } => rounds.clone(),
            PlanNodeType::Break { break_id, .. } => break_id.clone().into_iter().collect(),
        };

        out.push(self.tournament_id);

        out
    }
}