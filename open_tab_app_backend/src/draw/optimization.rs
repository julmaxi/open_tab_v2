use std::{collections::VecDeque, iter::zip};


use rand::{thread_rng, Rng};


use crate::draw_view::DrawBallot;



use super::evaluation::{DrawConstructionEvaluationContext, DrawEvaluator};


#[derive(Debug)]
struct Matrix {
    weights: Vec<f64>,
    num_options: usize,
    num_ballots: usize,
}

impl Matrix {
    fn new(num_options: usize, num_ballots: usize) -> Self {
        Self {
            weights: vec![0.0; num_options * num_ballots],
            num_options,
            num_ballots,
        }
    }

    #[allow(dead_code)]
    fn get(&self, option: usize, ballot: usize) -> f64 {
        self.weights[option * self.num_ballots + ballot]
    }

    fn get_option_ballots(&self, option: usize) -> &[f64] {
        &self.weights[option * self.num_ballots..(option + 1) * self.num_ballots]
    }

    fn set(&mut self, option: usize, ballot: usize, value: f64) {
        self.weights[option * self.num_ballots + ballot] = value;
    }
}

fn auction_algorithm(
    weights: Matrix,
) -> Vec<(usize, usize)> {
    let delta = 1.0 / (weights.num_ballots + 1) as f64;

    let mut queue = VecDeque::new();
    queue.extend(0..weights.num_options);

    let min_value = weights.weights.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let mut owners = vec![usize::MAX;weights.num_ballots];
    let mut prices = vec![*min_value - f64::EPSILON;weights.num_ballots];

    for _ in 0..10000 {
        if let Some(next) = queue.pop_front() {
            let bids = weights.get_option_ballots(next);

            let (best_bid, _) = zip(bids.iter(), prices.iter()).map(|(weight, price)| weight - price).enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).unwrap();

            if owners[best_bid] == usize::MAX {
                owners[best_bid] = next;
            } else {
                queue.push_back(owners[best_bid]);
                owners[best_bid] = next;
                prices[best_bid] += delta;
            }
        } else {
            break;
        }
    }
    
    owners.into_iter().enumerate().map(|(obj, owner)| (owner, obj)).collect()
}

pub(crate) fn find_best_ballot_assignments(ballots: &Vec<Vec<DrawBallot>>, evaluator: &DrawEvaluator<DrawConstructionEvaluationContext>, randomization_scale: f64) -> Result<Vec<DrawBallot>, anyhow::Error> {
    let mut rng = thread_rng();
    let mut matrix = Matrix::new(ballots.len(), ballots[0].len());
    for (option_idx, ballot_options) in ballots.iter().enumerate() {
        for (ballot_idx, ballot) in ballot_options.iter().enumerate() {
            let issues = evaluator.find_issues_in_ballot(ballot);
            let weight: f64 = issues.total_severity() as f64 + rng.gen_range(0.0..randomization_scale);

            matrix.set(
                option_idx,
                ballot_idx,
                -weight,
            );
        }
    }
    let solution = auction_algorithm(matrix);

    Ok(solution.iter().map(
        |(option_idx, ballot_idx)| {
            //TODO: Strictly speaking we never need to clone here, but the compiler doesn't know that
            //let ballot =             ballots[option_idx][*ballot_idx as usize].clone();
            //let issues = evaluator.find_issues_in_ballot(&ballot);
            //let weight: f64 = issues.total_severity() as f64;// + rng.gen_range(0.0..randomization_scale);

            /*if weight > 0.0 {
                dbg!(&solver.values()[option_idx as usize * ballots.len() + *ballot_idx as usize]);
                println!("Ballot {} has issues with total severity {}", ballot_idx, weight);
            }*/

            ballots[*option_idx][*ballot_idx as usize].clone()
        }
    ).collect())
}