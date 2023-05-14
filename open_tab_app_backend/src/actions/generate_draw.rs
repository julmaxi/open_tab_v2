use std::{error::Error, iter::zip};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::round::DrawType};

use sea_orm::prelude::*;

use crate::{draw::{PreliminaryRoundGenerator, PreliminariesDrawMode, evaluation::DrawEvaluator, preliminary::{RoundGenerationContext, DrawTeamInfo}}, TournamentParticipantsInfo};
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
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, Box<dyn Error>> where C: ConnectionTrait {
        let mut changes = EntityGroup::new();

        let all_rounds = TournamentRound::get_all_in_tournament(db, self.tournament_id).await?;

        let (rounds, other_rounds) : (Vec<_>, Vec<_>) = all_rounds.into_iter().partition(
            |r| self.draw_rounds.contains(&r.uuid)
        );

        if rounds.len() < self.draw_rounds.len() {
            return  Err(Box::new(GenerateDrawActionError::RoundIsNotInTournament { tournament_id: self.tournament_id }));
        }

        let prev_rounds_ballots = Ballot::get_all_in_rounds(db, other_rounds.iter().map(|r| r.uuid).collect()).await?;//.into_iter().into_group_map();

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

            let evaluator = DrawEvaluator::new_from_rounds(db, self.tournament_id, &other_rounds).await?;            let ballots = generator.generate_draw_for_rounds(&context, rounds.iter().collect(), &evaluator)?;
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
                        ballot_id: Uuid::nil()
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
                debate.ballot_id = real_ballot.uuid;

                changes.add(Entity::Ballot(real_ballot));
                changes.add(Entity::TournamentDebate(debate));
            }
        }

        Ok(changes)
    }
}