use std::{default, error::Error, fmt::Display};

use open_tab_entities::prelude::TournamentRound;
use rand::{thread_rng, seq::SliceRandom, Rng};
use sea_orm::prelude::Uuid;

use crate::{participants_list_view::{TeamEntry, ParticipantEntry}, draw_view::{DrawBallot, DrawTeam, DrawSpeaker}, tab_view::TeamRoundRole};

use itertools::Itertools;

use sparse_linear_assignment::{KhoslaSolver, AuctionSolver};

use super::evaluation::DrawEvaluator;


pub(crate) fn find_best_ballot_assignments(ballots: &Vec<Vec<DrawBallot>>, evaluator: &DrawEvaluator, randomization_scale: f64) -> Result<Vec<DrawBallot>, Box<dyn Error>> {
    let (mut solver, mut solution) = KhoslaSolver::new(ballots.len().into(), ballots[0].len().into(), (ballots.len() * ballots[0].len()).into());

    solver.init(ballots.len() as u32, ballots[0].len() as u32)?;
    let mut rng = thread_rng();
    for (option_idx, ballot_options) in ballots.iter().enumerate() {
        for (ballot_idx, ballot) in ballot_options.iter().enumerate() {
            let weight = evaluator.find_issues_in_ballot(ballot).total_severity() as f64 + (rng.gen::<f64>() * randomization_scale);

            solver.add_value(option_idx as u32, ballot_idx as u32, weight.into())?;
        }
    }
    solver.solve(&mut solution, false, None)?;

    Ok(solution.object_to_person.iter().enumerate().map(
        |(option_idx, ballot_idx)| {
            //TODO: Strictly speaking we never need to clone here, but the compiler doesn't know that
            ballots[option_idx][*ballot_idx as usize].clone()
        }
    ).collect())
}