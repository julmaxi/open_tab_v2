use core::num;
use std::{error::Error, fmt::{Display, Formatter}, collections::HashMap};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{round::DrawType, tournament_break::{TournamentBreak, TournamentBreakSourceRound, TournamentBreakSourceRoundType}}};

use sea_orm::prelude::*;

use crate::{draw_view::DrawBallot, participants_list_view::ParticipantEntry, draw::{PreliminaryRoundGenerator, PreliminariesDrawMode}};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTreeAction {
    tournament_id: Uuid,
    action: EditTreeActionType
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EditTreeActionType {
    AddThreePreliminaryRounds{ parent: Uuid },
    AddKOStage{ parent: Uuid, num_stages: u64 },
}


fn shift_round_indices(rounds: &mut Vec<TournamentRound>, start: u64, shift: u64) {
    for round in rounds.iter_mut() {
        if round.index >= start {
            round.index += shift;
        }
    }
}


use thiserror::Error;

#[derive(Error, Debug)]
pub enum EditTreeActionError {
    #[error("the parent round does not exist")]
    ParentRoundDoesNotExist {uuid: Uuid},
}

#[async_trait]
impl ActionTrait for EditTreeAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroups, Box<dyn Error>> where C: ConnectionTrait {
        let mut groups = EntityGroups::new();

        let all_existing_rounds = TournamentRound::get_all_in_tournament(db, self.tournament_id).await?;

        match self.action {
            EditTreeActionType::AddThreePreliminaryRounds { parent } => {
                let start_index = all_existing_rounds.iter().find(|r| r.uuid == parent).ok_or(
                    EditTreeActionError::ParentRoundDoesNotExist { uuid: parent }
                )?.index + 1;
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


                for index in 0..num_stages {
                    let round = TournamentRound {
                        uuid: Uuid::new_v4(),
                        tournament_id: self.tournament_id,
                        index: start_index + index,
                        draw_type: Some(DrawType::KnockoutDraw),
                    };

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
                    added_rounds.push(round);
                }

                added_breaks.into_iter().for_each(|b| groups.add(Entity::TournamentBreak(b)));
                added_rounds.into_iter().for_each(|r| groups.add(Entity::TournamentRound(r)));

                //generator.generate_draw_for_rounds(context, rounds, evaluator)
            }
        }

        Ok(groups)
    }
}