use std::collections::HashMap;

use itertools::Itertools;
use open_tab_entities::{prelude::{Ballot, Team}, domain::{round::{TeamDrawMode, SpeakerDrawMode}, tournament_plan_node::{TeamFoldMethod, TeamAssignmentRule, NonAlignedFoldMethod}}, tab::TeamRoundRole};
use rand::{thread_rng, seq::SliceRandom, Rng};
use sea_orm::prelude::Uuid;



#[derive(Debug, Clone)]
pub struct TeamPair {
    pub government_id: Uuid,
    pub opposition_id: Uuid,
}

impl TeamPair {
    pub fn shuffled(self) -> Self {
        let mut rng = thread_rng();
        if rng.gen_bool(0.5) {
            Self {
                government_id: self.opposition_id,
                opposition_id: self.government_id,
            }
        } else {
            self
        }
    }

    pub fn inverted(self) -> Self {
        Self {
            government_id: self.opposition_id,
            opposition_id: self.government_id,
        }
    }
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
    team_draw_mode: &TeamFoldMethod,
) -> Vec<TeamPair> {
    let mut rng = thread_rng();
    let pairs = match team_draw_mode {
        TeamFoldMethod::PowerPaired => {
            let gov_iter = breaking_teams.iter().step_by(2);
            let opp_iter = breaking_teams.iter().skip(1).step_by(2);
            gov_iter.zip(opp_iter).map(|(gov, opp)| TeamPair {
                government_id: *gov,
                opposition_id: *opp,
            }).collect_vec()        
        }
        TeamFoldMethod::InversePowerPaired => {
            reverse_fold(breaking_teams)        
        }
        TeamFoldMethod::BalancedPowerPaired => {
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
        TeamFoldMethod::Random => {
            let mut teams = breaking_teams.clone();
            teams.shuffle(&mut rng);
            teams.iter().step_by(2).zip(teams.iter().skip(1).step_by(2)).map(|(gov, opp)| TeamPair {
                government_id: *gov,
                opposition_id: *opp,
            }).collect_vec()
        }
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

pub fn pair_speakers(breaking_speakers: &Vec<Uuid>, speaker_draw_mode: &NonAlignedFoldMethod) -> Vec<Vec<Uuid>> {
    let mut rng = thread_rng();
    let pairs = match speaker_draw_mode {
        NonAlignedFoldMethod::TabOrder => {
            pair_consequtive_speakers(breaking_speakers)
        }
        NonAlignedFoldMethod::Random => {
            let mut speakers = breaking_speakers.clone();
            speakers.shuffle(&mut rng);
            pair_consequtive_speakers(&speakers)
        }
    };
    pairs
}

pub fn assign_teams(
    team_pairs: Vec<TeamPair>,
    config: &open_tab_entities::domain::tournament_plan_node::FoldDrawConfig,
    preceding_round_gov_opp_assignments: Option<&HashMap<Uuid, TeamRoundRole>>,
) -> Vec<TeamPair> {
    match config.team_assignment_rule {
        open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::Random => {
            team_pairs.into_iter().map(|p| p.shuffled()).collect_vec()
        },
        open_tab_entities::domain::tournament_plan_node::TeamAssignmentRule::InvertPrevious => {
            team_pairs.into_iter().map(|p| {
                let p = p.shuffled();
                if let Some(preceding_round_gov_opp_assignments) = &preceding_round_gov_opp_assignments {
                    let prev_gov_role = preceding_round_gov_opp_assignments.get(&p.government_id);
                    let prev_opp_role = preceding_round_gov_opp_assignments.get(&p.opposition_id);
                    if prev_gov_role == prev_gov_role  {
                        p.shuffled()
                    }
                    else {
                        if prev_gov_role == Some(&TeamRoundRole::Government) || prev_opp_role == Some(&TeamRoundRole::Opposition) {
                            p.inverted()
                        }
                        else {
                            p
                        }
        
                    }
                }
                else {
                    p.shuffled()
                }
            }).collect_vec()
        },
    }
}