use std::{error::Error, iter::zip};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{round::DrawType, tournament_break::TournamentBreak}};

use rand::seq::SliceRandom;
use sea_orm::prelude::*;

use crate::{draw::{PreliminaryRoundGenerator, PreliminariesDrawMode, evaluation::DrawEvaluator, preliminary::{RoundGenerationContext, DrawTeamInfo}, tab_draw::{pair_teams, pair_speakers, reverse_fold, pair_consequtive_speakers, TeamPair}}, TournamentParticipantsInfo, draw_view::{DrawBallot, DrawTeam, DrawSpeaker}};
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
    #[error("Can not draw round without draw mode")]
    CanNotDrawRoundWithoutDrawMode,
    #[error("Missing break for round")]
    MissingBreak
}

fn round_draw_from_team_and_speaker_pairs(team_pairs: Vec<TeamPair>, speaker_pairs: Vec<Vec<Uuid>>) -> Vec<DrawBallot> {
    team_pairs.into_iter().zip(speaker_pairs.into_iter()).map(
        |(team_pair, speaker_pair)| {
            let ballot = DrawBallot {
                uuid: Uuid::new_v4(),
                government: Some(
                    DrawTeam {
                        uuid: team_pair.government_id,
                        ..Default::default()
                    }
                ),
                opposition: Some(
                    DrawTeam {
                        uuid: team_pair.opposition_id,
                        ..Default::default()
                    }
                ),
                non_aligned_speakers: speaker_pair.into_iter().map(
                    |speaker_id| DrawSpeaker {
                        uuid: speaker_id,
                        ..Default::default()
                    }
                ).collect(),
                ..Default::default()
            };
            ballot
        }
    ).collect_vec()
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

        let _prev_rounds_ballots = Ballot::get_all_in_rounds(db, other_rounds.iter().map(|r| r.uuid).collect()).await?;//.into_iter().into_group_map();

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
            let round = rounds.first().unwrap();
            let round_break = TournamentBreak::get_break_for_round(db, round.uuid).await?;

            match &round.draw_type {
                Some(DrawType::TabDraw { config }) => {
                    let round_break = round_break.ok_or(GenerateDrawActionError::MissingBreak)?;
                    let team_pairs = pair_teams(&round_break.breaking_teams, config.team_draw, config.team_assignment_rule);
                    let speaker_pairs = pair_speakers(&round_break.breaking_speakers, config.speaker_draw);
                    let ballots = round_draw_from_team_and_speaker_pairs(team_pairs, speaker_pairs);
                    vec![ballots]
                },
                Some(DrawType::KnockoutDraw) => {
                    let round_break = round_break.ok_or(GenerateDrawActionError::MissingBreak)?;
                    let team_pairs = reverse_fold(&round_break.breaking_teams);
                    
                    let mut rng = rand::thread_rng();
                    let mut speakers = round_break.breaking_speakers.clone();
                    speakers.shuffle(&mut rng);
                    let speakers = pair_consequtive_speakers(&speakers);
                    let ballots = round_draw_from_team_and_speaker_pairs(team_pairs, speakers);
                    vec![ballots]
 
                },
                Some(_) => todo!(),
                None => return Err(Box::new(GenerateDrawActionError::CanNotDrawRoundWithoutDrawMode)),
            }
        }
        else if rounds.len() > 1 {
            if !rounds.iter().all(|r| r.draw_type == Some(DrawType::StandardPreliminaryDraw)) {
                return Err(Box::new(GenerateDrawActionError::CanOnlyDrawMultipleRoundsForStandardPreliminariesDraw))
            }

            let generator = PreliminaryRoundGenerator {
                draw_mode: PreliminariesDrawMode::AvoidClashes,
                randomization_scale: 0.5
            };

            let evaluator = DrawEvaluator::new_from_rounds(db, self.tournament_id, &other_rounds).await?;
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
                        ballot_id: Uuid::nil(),
                        is_motion_released_to_non_aligned: false,
                        venue_id: None
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