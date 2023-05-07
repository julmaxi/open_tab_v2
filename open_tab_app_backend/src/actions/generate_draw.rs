use std::{error::Error, fmt::{Display, Formatter}, collections::HashMap, iter::zip};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::round::{DrawType, self}};

use sea_orm::prelude::*;

use crate::{draw_view::DrawBallot, draw::{PreliminaryRoundGenerator, PreliminariesDrawMode, clashes::{ClashMap, ClashMapConfig}, evaluation::DrawEvaluator, preliminary::{RoundGenerationContext, DrawTeamInfo}}, TournamentParticipantsInfo};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateDrawAction {
    pub tournament_id: Uuid,
    pub draw_rounds: Vec<Uuid>
}

#[derive(Error, Debug)]
pub enum GenerateDrawActionError {
    #[error("Can only draw multiple rounds for standard preliminaries draw")]
    CanOnlyDrawMultipleRoundsForStandardPreliminariesDraw,
    #[error("Round is not in tournament {tournament_id}")]
    RoundIsNotInTournament { tournament_id: Uuid },
}

#[async_trait]
impl ActionTrait for GenerateDrawAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroups, Box<dyn Error>> where C: ConnectionTrait {
        let mut changes = EntityGroups::new();

        //let rounds = TournamentRound::get_many(db, self.draw_rounds).await?.into_iter().sorted_by_key(|r| r.index).collect_vec();

        let all_rounds = TournamentRound::get_all_in_tournament(db, self.tournament_id).await?;

        let (rounds, other_rounds) : (Vec<_>, Vec<_>) = all_rounds.into_iter().partition(
            |r| self.draw_rounds.contains(&r.uuid)
        );

        if rounds.len() < self.draw_rounds.len() {
            return  Err(Box::new(GenerateDrawActionError::RoundIsNotInTournament { tournament_id: self.tournament_id }));
        }

        let prev_rounds_ballots = Ballot::get_all_in_rounds(db, other_rounds.iter().map(|r| r.uuid).collect()).await?.into_iter().into_group_map();

        let tournament_info = TournamentParticipantsInfo::load(db, self.tournament_id).await?;

        let context = RoundGenerationContext {
            teams: tournament_info.team_members.iter().map(|(uuid, members)| 
                DrawTeamInfo {
                    uuid: *uuid,
                    member_ids: members.clone()
                }
            ).collect(),
            adjudicators: tournament_info.participants_by_id.values().filter_map(
                |a| match a.role {
                    ParticipantRole::Adjudicator(_) => Some(a.uuid),
                    _ => None
                }
            ).collect(),
        };

        let ballots = if rounds.len() == 1 {
            todo!();
        }
        else if rounds.len() > 1 {
            if !rounds.iter().all(|r| r.draw_type == Some(DrawType::StandardPreliminaryDraw)) {
                return Err(Box::new(GenerateDrawActionError::CanOnlyDrawMultipleRoundsForStandardPreliminariesDraw))
            }

            let generator = PreliminaryRoundGenerator {
                draw_mode: PreliminariesDrawMode::AvoidClashes,
                randomization_scale: 0.5
            };

            let mut clash_map = ClashMap::new_for_tournament(
                ClashMapConfig::default(),
                self.tournament_id,
                db
            ).await?;

            clash_map.add_dynamic_clashes_from_round_ballots(prev_rounds_ballots.iter().collect(), &tournament_info.team_members)?;

            let evaluator = DrawEvaluator::new(
                clash_map,
                Default::default()
            );

            let ballots = generator.generate_draw_for_rounds(&context, rounds.iter().collect(), &evaluator)?;
            ballots

        } else {
            vec![]
        };

        let existing_debates = TournamentDebate::get_all_in_rounds(db, rounds.iter().map(|r| r.uuid).collect()).await?;

        for (round, round_existing_debates, round_new_ballots) in izip![rounds.iter(), existing_debates.into_iter(), ballots.into_iter()] {
            let debates = if round_existing_debates.len() < round_new_ballots.len() {
                let new_debates = (round_existing_debates.len()..round_new_ballots.len()).map(
                    |index| TournamentDebate {
                        uuid: Uuid::new_v4(),
                        round_id: round.uuid,
                        index: index as u64,
                        current_ballot_uuid: Uuid::nil()
                    }
                );
                round_existing_debates.into_iter().chain(
                    new_debates
                ).collect_vec()
            }
            else {
                round_existing_debates
            };

            for (mut debate, ballot) in zip(debates.into_iter(), round_new_ballots.into_iter()) {
                let mut real_ballot : Ballot = ballot.into();
                real_ballot.uuid = Uuid::new_v4();
                debate.current_ballot_uuid = real_ballot.uuid;

                changes.add(Entity::Ballot(real_ballot));
                changes.add(Entity::TournamentDebate(debate));
            }
        }

        Ok(changes)
    }
}