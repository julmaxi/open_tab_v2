use std::error::Error;

use itertools::Itertools;
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{round::DrawType, tournament_break::{TournamentBreak, TournamentBreakSourceRound, TournamentBreakSourceRoundType}}};

use sea_orm::prelude::*;

use crate::draw::preliminary::MinorBreakRoundDrawType;
use serde::{Serialize, Deserialize};
use thiserror::Error;

use super::ActionTrait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTreeAction {
    tournament_id: Uuid,
    action: EditTreeActionType
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EditTreeActionType {
    AddThreePreliminaryRounds { parent: Option<Uuid> },
    AddMinorBreakRounds { parent: Uuid, draws: Vec<MinorBreakRoundDrawType> },
    AddTimBreakRounds { parent: Uuid },
    AddKOStage { parent: Uuid, num_stages: u64 },
}

#[derive(Error, Debug)]
pub enum EditTreeActionError {
    #[error("the parent round does not exist")]
    ParentRoundDoesNotExist {uuid: Uuid},
}

#[async_trait]
impl ActionTrait for EditTreeAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, Box<dyn Error>> where C: ConnectionTrait {
        let mut groups = EntityGroup::new();

        let all_existing_rounds = TournamentRound::get_all_in_tournament(db, self.tournament_id).await?;

        match self.action {
            EditTreeActionType::AddThreePreliminaryRounds { parent } => {
                let start_index = if let Some(parent) = parent {
                    all_existing_rounds.iter().find(|r| r.uuid == parent).ok_or(
                        EditTreeActionError::ParentRoundDoesNotExist { uuid: parent }
                    )?.index + 1
                }
                else {
                    0
                };
                all_existing_rounds.iter().for_each(|r| {
                    if r.index >= start_index {
                        let mut r = r.clone();
                        r.index += 3;
                        groups.add(Entity::TournamentRound(r));
                    }
                });

                let rounds = (start_index..(start_index + 3)).map(
                    |index| TournamentRound {
                        uuid: Uuid::new_v4(),
                        tournament_id: self.tournament_id,
                        index,
                        draw_type: Some(DrawType::StandardPreliminaryDraw),
                        ..Default::default()
                    }
                ).collect_vec();

                rounds.into_iter().for_each(|r| groups.add(Entity::TournamentRound(r)));

                //generator.generate_draw_for_rounds(context, rounds, evaluator)
            },
            EditTreeActionType::AddKOStage { parent, num_stages } => {
                let start_index = all_existing_rounds.iter().find(|r| r.uuid == parent).ok_or(
                    EditTreeActionError::ParentRoundDoesNotExist { uuid: parent }
                )?.index + 1;
                all_existing_rounds.iter().for_each(|r| {
                    if r.index >= start_index {
                        let mut r = r.clone();
                        r.index += num_stages;
                        groups.add(Entity::TournamentRound(r));
                    }
                });

                let all_source_uuids = all_existing_rounds.iter().filter(
                    |r| r.index < start_index
                ).map(|r| r.uuid).collect_vec();

                let mut added_breaks = vec![];
                let mut added_rounds : Vec<TournamentRound> = vec![];

                let mut tab_break = TournamentBreak::new(self.tournament_id, open_tab_entities::domain::tournament_break::BreakType::TabBreak { num_debates: 
                    (2 as u32).pow(num_stages as u32) as u16
                });
                tab_break.source_rounds.extend(
                    all_source_uuids.iter().map(|uuid| TournamentBreakSourceRound {
                        uuid: *uuid,
                        break_type: TournamentBreakSourceRoundType::Tab
                }));

                for index in 0..num_stages {
                    let round = TournamentRound {
                        uuid: Uuid::new_v4(),
                        tournament_id: self.tournament_id,
                        index: start_index + index,
                        draw_type: Some(DrawType::KnockoutDraw),
                        ..Default::default()
                    };

                    (0..(2 as u32).pow((num_stages - index - 1) as u32)).for_each(
                        |room_index| {
                            let mut ballot = Ballot::default();
                            ballot.uuid = Uuid::new_v4();
                            let debate = TournamentDebate {
                                uuid: Uuid::new_v4(),
                                round_id: round.uuid,
                                index: room_index as u64,
                                ballot_id: ballot.uuid,
                            };
                            groups.add(Entity::TournamentDebate(debate));
                            groups.add(Entity::Ballot(ballot));
                        }
                    );

                    if index != 0 {
                        let mut break_ = TournamentBreak::new(self.tournament_id, open_tab_entities::domain::tournament_break::BreakType::KOBreak);
                        break_.source_rounds.extend(
                            all_source_uuids.iter().map(|uuid| TournamentBreakSourceRound {
                                uuid: *uuid,
                                break_type: TournamentBreakSourceRoundType::Tab
                            })
                        );
    
                        if added_rounds.len() > 0 {
                            break_.source_rounds.push(TournamentBreakSourceRound {
                                uuid: added_rounds.last().map(|r| r.uuid).unwrap(),
                                break_type: TournamentBreakSourceRoundType::Knockout
                            });    
                        }
    
                        if added_rounds.len() > 1 {
                            let tab_rounds = added_rounds[
                                0..(added_rounds.len() - 1)
                            ].iter().map(|r| r.uuid).collect_vec();
    
                            break_.source_rounds.extend(
                                tab_rounds.iter().map(|uuid| TournamentBreakSourceRound {
                                    uuid: *uuid,
                                    break_type: TournamentBreakSourceRoundType::Tab
                                })
                            );
                        }
    
                        break_.child_rounds.push(round.uuid);
    
                        added_breaks.push(break_);    
                    }
                    else {
                        tab_break.child_rounds.push(round.uuid);
                    }
                    added_rounds.push(round);
                }
                added_breaks.push(tab_break);

                added_breaks.into_iter().for_each(|b| groups.add(Entity::TournamentBreak(b)));
                added_rounds.into_iter().for_each(|r| groups.add(Entity::TournamentRound(r)));

                //generator.generate_draw_for_rounds(context, rounds, evaluator)
            },
            EditTreeActionType::AddMinorBreakRounds { parent, draws } => {
                let mut rounds = vec![];

                let start_index = all_existing_rounds.iter().find(|r| r.uuid == parent).ok_or(
                    EditTreeActionError::ParentRoundDoesNotExist { uuid: parent }
                )?.index + 1;
                all_existing_rounds.iter().for_each(|r| {
                    if r.index >= start_index {
                        let mut r = r.clone();
                        r.index += draws.len() as u64;
                        groups.add(Entity::TournamentRound(r));
                    }
                });

                let all_source_uuids = all_existing_rounds.iter().filter(
                    |r| r.index < start_index
                ).map(|r| r.uuid).collect_vec();

                let mut break_ = TournamentBreak::new(self.tournament_id, open_tab_entities::domain::tournament_break::BreakType::TwoThirdsBreak);
                break_.source_rounds.extend(
                    all_source_uuids.iter().map(|uuid| TournamentBreakSourceRound {
                        uuid: *uuid,
                        break_type: TournamentBreakSourceRoundType::Tab
                    })
                );

                for (idx, draw) in draws.iter().enumerate() {
                    let draw_type : DrawType = draw.clone().into();
                    let round = TournamentRound {
                        uuid: Uuid::new_v4(),
                        tournament_id: self.tournament_id,
                        index: start_index + idx as u64,
                        draw_type: Some(draw_type),
                        ..Default::default()
                    };

                    break_.child_rounds.push(round.uuid);

                    rounds.push(round);
                }

                rounds.into_iter().for_each(|r| groups.add(Entity::TournamentRound(r)));
                groups.add(Entity::TournamentBreak(break_));
            },
            EditTreeActionType::AddTimBreakRounds { parent } => {
                let start_index = all_existing_rounds.iter().find(|r| r.uuid == parent).ok_or(
                    EditTreeActionError::ParentRoundDoesNotExist { uuid: parent }
                )?.index + 1;
                all_existing_rounds.iter().for_each(|r| {
                    if r.index >= start_index {
                        let mut r = r.clone();
                        r.index += 2;
                        groups.add(Entity::TournamentRound(r));
                    }
                });

                let all_source_uuids = all_existing_rounds.iter().filter(
                    |r| r.index < start_index
                ).map(|r| r.uuid).collect_vec();

                let mut break_after_tab = TournamentBreak::new(self.tournament_id, open_tab_entities::domain::tournament_break::BreakType::TwoThirdsBreak);
                break_after_tab.source_rounds.extend(
                    all_source_uuids.iter().map(|uuid| TournamentBreakSourceRound {
                        uuid: *uuid,
                        break_type: TournamentBreakSourceRoundType::Tab
                    })
                );
                let first_round = TournamentRound {
                    uuid: Uuid::new_v4(),
                    tournament_id: self.tournament_id,
                    index: start_index,
                    draw_type: Some(DrawType::Randomized),
                    ..Default::default()
                };

                break_after_tab.child_rounds.push(first_round.uuid);

                let mut second_break = TournamentBreak::new(self.tournament_id, open_tab_entities::domain::tournament_break::BreakType::TimBreak);
                second_break.source_rounds.extend(
                    all_source_uuids.iter().map(|uuid| TournamentBreakSourceRound {
                        uuid: *uuid,
                        break_type: TournamentBreakSourceRoundType::Tab
                    })
                );
                second_break.source_rounds.push(
                    TournamentBreakSourceRound {
                        uuid: first_round.uuid,
                        break_type: TournamentBreakSourceRoundType::Tab
                    }
                );

                let second_round = TournamentRound {
                    uuid: Uuid::new_v4(),
                    tournament_id: self.tournament_id,
                    index: start_index + 1,
                    draw_type: Some(DrawType::BalancedRandomized),
                    ..Default::default()
                };

                second_break.child_rounds.push(second_round.uuid);
                groups.add(Entity::TournamentBreak(break_after_tab));
                groups.add(Entity::TournamentRound(first_round));
                groups.add(Entity::TournamentBreak(second_break));
                groups.add(Entity::TournamentRound(second_round));
            },
        }

        Ok(groups)
    }
}