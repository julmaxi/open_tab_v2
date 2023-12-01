use std::collections::HashMap;

use itertools::Itertools;
use open_tab_entities::{domain::tournament_plan_node::{TeamFoldMethod, NonAlignedFoldMethod}, tab::TeamRoundRole};
use rand::{thread_rng, seq::SliceRandom, Rng};
use sea_orm::prelude::Uuid;



#[derive(Debug, Clone, PartialEq, Eq)]
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
            let lower_half = breaking_teams.iter().rev().take(half_team_count).rev();
            let remainder: Vec<&Uuid> = if breaking_teams.len() % 4 != 0 {
                breaking_teams.iter().skip(half_team_count).take(2).collect()
            }
            else {
                vec![]
            };

            let upper_pairs = reverse_fold(&upper_half.map(|u| *u).collect());
            let lower_pairs = reverse_fold(&lower_half.map(|u| *u).collect());
            let center_pair = if remainder.len() > 0 {
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
        TeamFoldMethod::HalfRandom => {
            let teams = breaking_teams.clone();

            let mut half_team_count = breaking_teams.len() / 2;

            if half_team_count % 2 != 0 {
                half_team_count -= 1;
            }

            let mut upper_half = teams.iter().take(half_team_count).collect_vec();
            let mut lower_half = teams.iter().rev().take(half_team_count).collect_vec();

            let remainder: Vec<&Uuid> = if teams.len() % 4 != 0 {
                breaking_teams.iter().skip(half_team_count).take(2).collect()
            }
            else {
                vec![]
            };

            upper_half.shuffle(&mut rng);
            lower_half.shuffle(&mut rng);

            dbg!(&upper_half.iter().step_by(2).zip(upper_half.iter().skip(1).step_by(2)).collect_vec());
            let upper_half = upper_half.iter().step_by(2).zip(upper_half.iter().skip(1).step_by(2)).map(|(gov, opp)| TeamPair {
                government_id: **gov,
                opposition_id: **opp,
            });
            let remaining_pair = if remainder.len() > 0 {
                vec![
                    TeamPair {
                        government_id: *remainder[0],
                        opposition_id: *remainder[1],
                    }
                ]
            }
            else {
                vec![]
            };
            let lower_half = lower_half.iter().step_by(2).zip(lower_half.iter().skip(1).step_by(2)).map(|(gov, opp)| TeamPair {
                government_id: **gov,
                opposition_id: **opp,
            });

            upper_half.chain(remaining_pair.into_iter()).chain(lower_half).collect_vec()
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

#[cfg(test)]
mod test {
    use itertools::Itertools;
    use open_tab_entities::domain::tournament_plan_node::TeamFoldMethod;
    use sea_orm::prelude::Uuid;

    use super::TeamPair;

    fn get_teams(n: usize) -> Vec<Uuid> {
        (0..n).map(|i| Uuid::from_u128(i as u128)).collect()
    }

    fn pairs_to_team_pairs(pairs: Vec<(i16, i16)>) -> Vec<TeamPair> {
        pairs.iter().flat_map(|p| vec![TeamPair { government_id: Uuid::from_u128(p.0 as u128), opposition_id: Uuid::from_u128(p.1 as u128) } ]).collect()
    }

    #[test]
    fn test_power_pairing() {
        let teams = get_teams(12);
        let pairs = super::pair_teams(&teams, &TeamFoldMethod::PowerPaired);
        assert_eq!(pairs, pairs_to_team_pairs(
            vec![
                (0, 1),
                (2, 3),
                (4, 5),
                (6, 7),
                (8, 9),
                (10, 11),
            ]
        ));
    }

    #[test]
    fn test_inverse_power_pairing() {
        let teams = get_teams(12);
        let pairs = super::pair_teams(&teams, &TeamFoldMethod::InversePowerPaired);
        assert_eq!(pairs, pairs_to_team_pairs(
            vec![
                (0, 11),
                (1, 10),
                (2, 9),
                (3, 8),
                (4, 7),
                (5, 6),
            ]
        ));
    }

    #[test]
    fn test_balanced_power_pairing_even_rooms() {
        let teams = get_teams(12);
        let pairs = super::pair_teams(&teams, &TeamFoldMethod::BalancedPowerPaired);
        assert_eq!(pairs, pairs_to_team_pairs(
            vec![
                (0, 5),
                (1, 4),
                (2, 3),
                (6, 11),
                (7, 10),
                (8, 9),
            ]
        ));
    }

    #[test]
    fn test_balanced_power_pairing_uneven_rooms() {
        let teams = get_teams(14);
        let pairs = super::pair_teams(&teams, &TeamFoldMethod::BalancedPowerPaired);
        assert_eq!(pairs, pairs_to_team_pairs(
            vec![
                (0, 5),
                (1, 4),
                (2, 3),
                (6, 7),
                (8, 13),
                (9, 12),
                (10, 11),
            ]
        ));
    }

    #[test]
    fn test_half_random_even_rooms() {
        let teams = get_teams(12);
        let pairs = super::pair_teams(&teams, &TeamFoldMethod::HalfRandom);
        let mut upper_half_pairs_teams : Vec<_> = pairs.iter().take(3).flat_map(|p| vec![p.government_id, p.opposition_id].into_iter()).collect();
        let mut lower_half_pairs_teams : Vec<_> = pairs.iter().skip(3).take(3).flat_map(|p| vec![p.government_id, p.opposition_id].into_iter()).collect();
        upper_half_pairs_teams.sort();
        lower_half_pairs_teams.sort();

        assert_eq!(upper_half_pairs_teams, (0..6).into_iter().map(Uuid::from_u128).collect_vec());
        assert_eq!(lower_half_pairs_teams, (6..12).into_iter().map(Uuid::from_u128).collect_vec());
    }

    #[test]
    fn test_half_random_uneven_rooms() {
        let teams = get_teams(14);
        let pairs = super::pair_teams(&teams, &TeamFoldMethod::HalfRandom);
        let mut upper_half_pairs_teams : Vec<_> = pairs.iter().take(3).flat_map(|p| vec![p.government_id, p.opposition_id].into_iter()).collect();
        let mut lower_half_pairs_teams : Vec<_> = pairs.iter().rev().take(3).flat_map(|p| vec![p.government_id, p.opposition_id].into_iter()).collect();

        let center_teams = pairs.iter().skip(3).take(1).flat_map(|p| vec![p.government_id, p.opposition_id].into_iter()).collect_vec();
        upper_half_pairs_teams.sort();
        lower_half_pairs_teams.sort();

        assert_eq!(upper_half_pairs_teams, (0..6).into_iter().map(Uuid::from_u128).collect_vec());
        assert_eq!(lower_half_pairs_teams, (8..14).into_iter().map(Uuid::from_u128).collect_vec());
        assert_eq!(center_teams, (6..8).into_iter().map(Uuid::from_u128).collect_vec());
    }
}