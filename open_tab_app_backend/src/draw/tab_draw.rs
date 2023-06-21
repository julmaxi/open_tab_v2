use itertools::Itertools;
use open_tab_entities::{prelude::Ballot, domain::round::{TeamDrawMode, TeamAssignmentRule, SpeakerDrawMode}};
use rand::{thread_rng, seq::SliceRandom, Rng};
use sea_orm::prelude::Uuid;



pub struct TeamPair {
    pub government_id: Uuid,
    pub opposition_id: Uuid,
}

pub fn reverse_fold(
    items: &Vec<Uuid>,
) -> Vec<TeamPair> {
    let left = items.iter().take(items.len() / 2);
    let right = items.iter().rev().take(items.len() / 2);
    left.zip(right).map(|(gov, opp)| TeamPair {
        government_id: *gov,
        opposition_id: *opp,
    }).collect_vec()
}

pub fn pair_teams(
    breaking_teams: &Vec<Uuid>,
    team_draw_mode: TeamDrawMode,
    team_assignment_rule: TeamAssignmentRule,
) -> Vec<TeamPair> {
    assert!(breaking_teams.len() % 2 == 0, "Breaking teams must be even");
    let mut rng = thread_rng();
    let mut pairs = match team_draw_mode {
        TeamDrawMode::PowerPaired => {
            let gov_iter = breaking_teams.iter().step_by(2);
            let opp_iter = breaking_teams.iter().skip(1).step_by(2);
            gov_iter.zip(opp_iter).map(|(gov, opp)| TeamPair {
                government_id: *gov,
                opposition_id: *opp,
            }).collect_vec()        
        }
        TeamDrawMode::InversePowerPaired => {
            reverse_fold(breaking_teams)        
        }
        TeamDrawMode::BalancedPowerPaired => {
            let mut half_team_count = breaking_teams.len() / 2;

            if half_team_count % 2 != 0 {
                half_team_count -= 1;
            }

            let upper_half = breaking_teams.iter().take(half_team_count);
            let lower_half = breaking_teams.iter().rev().take(half_team_count);
            let remainder = if half_team_count % 2 != 0 {
                breaking_teams.iter().skip(half_team_count).take(2).collect()
            }
            else {
                vec![]
            };

            let upper_pairs = reverse_fold(&upper_half.map(|u| *u).collect());
            let lower_pairs = reverse_fold(&lower_half.map(|u| *u).collect());
            let center_pair = if half_team_count % 2 != 0 {
                let gov = remainder[0];
                let opp = remainder[1];
                vec![TeamPair {
                    government_id: *gov,
                    opposition_id: *opp,
                }]
            } else {
                vec![]
            };
            
            upper_pairs.into_iter().chain(center_pair.into_iter()).chain(lower_pairs.into_iter()).collect_vec()
        }
        TeamDrawMode::Random => {
            let mut teams = breaking_teams.clone();
            teams.shuffle(&mut rng);
            teams.iter().step_by(2).zip(teams.iter().skip(1).step_by(2)).map(|(gov, opp)| TeamPair {
                government_id: *gov,
                opposition_id: *opp,
            }).collect_vec()
        }
    };

    let pairs = match team_assignment_rule {
        TeamAssignmentRule::Random => {
            pairs.into_iter().map(
                |pair| {
                    let mut rng = thread_rng();
                    if rng.gen() {
                        pair
                    } else {
                        TeamPair {
                            government_id: pair.opposition_id,
                            opposition_id: pair.government_id,
                        }
                    }
                }
            ).collect_vec()
        }
        TeamAssignmentRule::Fixed => pairs
    };
    pairs
}

pub fn pair_consequtive_speakers(speakers: &Vec<Uuid>) -> Vec<Vec<Uuid>> {
    let it1: std::iter::StepBy<std::slice::Iter<Uuid>> = speakers.iter().step_by(3);
    let it2 = speakers.iter().skip(1).step_by(3);
    let it3 = speakers.iter().skip(2).step_by(3);

    let pairs = it1.zip(it2).zip(it3).map(|((s1, s2), s3)| {
        vec![*s1, *s2, *s3]
    }).collect_vec();
    pairs

}

pub fn pair_speakers(breaking_speakers: &Vec<Uuid>, speaker_draw_mode: SpeakerDrawMode) -> Vec<Vec<Uuid>> {
    let mut rng = thread_rng();
    let pairs = match speaker_draw_mode {
        SpeakerDrawMode::PowerPaired => {
            pair_consequtive_speakers(breaking_speakers)
        }
        SpeakerDrawMode::Random => {
            let mut speakers = breaking_speakers.clone();
            speakers.shuffle(&mut rng);
            pair_consequtive_speakers(&speakers)
        }
    };
    pairs
}