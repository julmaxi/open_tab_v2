use std::collections::HashMap;

use itertools::Itertools;
use sea_orm::prelude::Uuid;
use serde::{Serialize, Deserialize};
use crate::draw_view::DrawBallot;

use super::clashes::{ClashMap, ClashType};


#[derive(Debug, Clone)]
pub struct DrawEvaluator {
    pub clash_map: ClashMap,
    config: DrawEvaluatorConfig
}

#[derive(Debug, Clone)]
pub struct DrawEvaluatorConfig {
    pub adj_adj_clash_factor: f32,
    pub adj_team_clash_factor: f32,
    pub adj_speaker_clash_factor: f32,
    pub team_team_clash_factor: f32,
    pub team_speaker_clash_factor: f32,
    pub speaker_speaker_clash_factor: f32,
    pub adj_adj_repeat_clash_severity: u16,
    pub adj_team_repeat_clash_severity: u16,
    pub adj_non_aligned_speaker_repeat_clash_severity: u16,
    pub team_team_repeat_clash_severity: u16,
    pub team_speaker_repeat_clash_severity: u16,
    pub non_aligned_speakers_repeat_clash_severity: u16,
}

impl Default for DrawEvaluatorConfig {
    fn default() -> Self {
        DrawEvaluatorConfig {
            adj_adj_clash_factor: 0.3,
            adj_team_clash_factor: 1.0,
            adj_speaker_clash_factor: 0.5,
            team_team_clash_factor: 0.2,
            team_speaker_clash_factor: 0.1,
            speaker_speaker_clash_factor: 0.1,
            adj_adj_repeat_clash_severity: 40,
            adj_team_repeat_clash_severity: 40,
            adj_non_aligned_speaker_repeat_clash_severity: 40,
            team_team_repeat_clash_severity: 10,
            team_speaker_repeat_clash_severity: 10,
            non_aligned_speakers_repeat_clash_severity: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DrawIssueTarget {
    Adjudicator(Uuid),
    Speaker(Uuid),
    Team{team_id: Uuid, involved_speakers: Vec<Uuid>}
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DrawIssue {
    #[serde(flatten)]
    pub issue_type: ClashType,
    pub severity: u16,
    pub target: DrawIssueTarget
}

pub struct BallotEvaluationResult {
    pub government_issues: Vec<DrawIssue>,
    pub opposition_issues: Vec<DrawIssue>,
    pub non_aligned_issues: HashMap<Uuid, Vec<DrawIssue>>,
    pub adjudicator_issues: HashMap<Uuid, Vec<DrawIssue>>
}

impl BallotEvaluationResult {
    pub fn new() -> Self {
        BallotEvaluationResult {
            government_issues: Vec::new(),
            opposition_issues: Vec::new(),
            non_aligned_issues: HashMap::new(),
            adjudicator_issues: HashMap::new()
        }
    }
}

impl BallotEvaluationResult {
    pub fn total_severity(&self) -> u32 {
        self.government_issues.iter().map(|i| i.severity as u32).sum::<u32 >()
            + self.opposition_issues.iter().map(|i| i.severity as u32).sum::<u32 >()
            + self.non_aligned_issues.iter().map(|(_, issues)| issues.iter().map(|i| i.severity as u32 ).sum::<u32 >()).sum::<u32 >()
            + self.adjudicator_issues.iter().map(|(_, issues)| issues.iter().map(|i| i.severity as u32 ).sum::<u32 >()).sum::<u32 >()
    }
}


impl DrawEvaluator {
    pub fn new(clash_map: ClashMap, config: DrawEvaluatorConfig) -> Self {
        DrawEvaluator {
            clash_map,
            config
        }
    }

    pub fn get_base_severity(&self, clash_type: &ClashType) -> u16 {
        match clash_type {
            ClashType::TeamSpeakerHasSeenTeamSpeaker{..} => self.config.team_team_repeat_clash_severity,
            ClashType::TeamSpeakerHasSeenNonAlignedSpeaker{..} => self.config.team_speaker_repeat_clash_severity,
            ClashType::NonAlignedSpeakerHasSeenNonAlignedSpeaker{..} => self.config.non_aligned_speakers_repeat_clash_severity,
            ClashType::JudgeHasSeenTeamSpeaker{..} => self.config.adj_team_repeat_clash_severity,
            ClashType::JudgeHasSeenNonAlignedSpeaker{..} => self.config.adj_non_aligned_speaker_repeat_clash_severity,
            ClashType::JudgeHasSeenJudge{..} => self.config.adj_adj_repeat_clash_severity,
            ClashType::DeclaredClash{severity} => *severity,
            ClashType::InstitutionalClash{severity, ..} => *severity,
            ClashType::SameTeamClash => 1
        }
    }

    pub fn find_issues_in_ballot(&self, ballot: &DrawBallot) -> BallotEvaluationResult {
        let gov_member_ids = ballot.government.as_ref().iter().flat_map(|t| t.members.iter().map(|s| s.uuid)).collect::<Vec<_>>();
        let opp_member_ids = ballot.opposition.as_ref().iter().flat_map(|t| t.members.iter().map(|s| s.uuid)).collect::<Vec<_>>();
        let non_aligned_ids = ballot.non_aligned_speakers.iter().map(|s| s.uuid).collect::<Vec<_>>();
        let adjudicator_ids = ballot.adjudicators.iter().map(|a| a.uuid).collect::<Vec<_>>();

        //let mut issues = HashMap::new();
        let mut issues = BallotEvaluationResult::new();

        for adj_pair in adjudicator_ids.iter().combinations(2) {
            let adj_1_id = adj_pair[0];
            let adj_2_id = adj_pair[1];
            let adj_clashes = self.clash_map.get_clashes_for_participant(adj_1_id);

            // The clash map is symmetric, so we only need to check one direction
            let clashes = adj_clashes.get(adj_2_id).iter().map(|c| c.iter()).flatten().collect_vec();
            for clash in clashes {
                let severity = (self.get_base_severity(&clash.clash_type) as f32 * self.config.adj_adj_clash_factor) as u16;
                issues.adjudicator_issues.entry(*adj_1_id).or_insert_with(Vec::new).push(DrawIssue {
                    issue_type: clash.clash_type.clone(),
                    severity: severity,
                    target: DrawIssueTarget::Adjudicator(*adj_2_id)
                });
                issues.adjudicator_issues.entry(*adj_2_id).or_insert_with(Vec::new).push(DrawIssue {
                    issue_type: clash.clash_type.clone(),
                    severity: severity,
                    target: DrawIssueTarget::Adjudicator(*adj_1_id)
                });
            }
        }

        for (adj_id, speaker_id) in adjudicator_ids.iter().cartesian_product(non_aligned_ids.iter()) {
            let adj_clashes = self.clash_map.get_clashes_for_participant(adj_id);
            let clashes = adj_clashes.get(speaker_id).iter().map(|c| c.iter()).flatten().collect_vec();
            for clash in clashes {
                let severity = (self.get_base_severity(&clash.clash_type) as f32 * self.config.adj_speaker_clash_factor) as u16;
                issues.adjudicator_issues.entry(*adj_id).or_insert_with(Vec::new).push(DrawIssue {
                    issue_type: clash.clash_type.clone(),
                    severity: severity,
                    target: DrawIssueTarget::Speaker(*speaker_id)
                });
                issues.non_aligned_issues.entry(*speaker_id).or_insert_with(Vec::new).push(DrawIssue {
                    issue_type: clash.clash_type.clone(),
                    severity: severity,
                    target: DrawIssueTarget::Adjudicator(*adj_id)
                });
            }
        }

        for adj_id in adjudicator_ids.iter() {
            let adj_clashes = self.clash_map.get_clashes_for_participant(adj_id);

            vec![(&ballot.government, &gov_member_ids), (&ballot.opposition, &opp_member_ids)].into_iter().map(|(team_id, member_ids)| {
                member_ids.iter().flat_map(
                    |member_id| {
                        adj_clashes.get(member_id).iter().map(|cs| 
                            cs.iter().map(
                                |c| {
                                    DrawIssue {
                                        issue_type: c.clash_type.clone(),
                                        severity: (self.get_base_severity(&c.clash_type) as f32 * self.config.adj_team_clash_factor) as u16,
                                        target: DrawIssueTarget::Team{team_id: team_id.as_ref().unwrap().uuid, involved_speakers: vec![*member_id]}
                                    }
                                }
                            ).collect_vec()
                        ).collect_vec()
                    }
                ).flatten().sorted().coalesce(coalesce_issues).collect_vec()
            }).flatten().for_each(|issue| {
                issues.adjudicator_issues.entry(*adj_id).or_insert_with(Vec::new).push(issue.clone());
                match &issue.target {
                    DrawIssueTarget::Team { team_id, .. } => {
                        if *team_id == ballot.government.as_ref().map(|t| t.uuid).unwrap_or(Uuid::nil()) {
                            issues.government_issues.push(DrawIssue {target: DrawIssueTarget::Adjudicator(*adj_id), ..issue});
                        } else if *team_id == ballot.opposition.as_ref().map(|t| t.uuid).unwrap_or(Uuid::nil()) {
                            issues.opposition_issues.push(DrawIssue {target: DrawIssueTarget::Adjudicator(*adj_id), ..issue});
                        } else {
                            unreachable!()
                        }
                    },
                    _ => unreachable!()
                }
            });
        }

        for non_aligned_id in non_aligned_ids.iter() {
            let non_aligned_clashes = self.clash_map.get_clashes_for_participant(non_aligned_id);
            vec![(&ballot.government, &gov_member_ids), (&ballot.opposition, &opp_member_ids)].into_iter().map(|(team_id, member_ids)| {
                member_ids.iter().flat_map(|member_id| {
                    non_aligned_clashes.get(member_id).iter().map(|cs| 
                        cs.iter().map(
                            |c| {
                                DrawIssue {
                                    issue_type: c.clash_type.clone(),
                                    severity: (self.get_base_severity(&c.clash_type) as f32 * self.config.team_speaker_clash_factor) as u16,
                                    target: DrawIssueTarget::Team{team_id: team_id.as_ref().unwrap().uuid, involved_speakers: vec![*member_id]}
                                }
                            }
                        ).collect_vec()
                    ).collect_vec()
                }).flatten().sorted().coalesce(coalesce_issues).collect_vec()
            }).flatten().for_each(|issue| {
                issues.non_aligned_issues.entry(*non_aligned_id).or_insert_with(Vec::new).push(issue.clone());
                match &issue.target {
                    DrawIssueTarget::Team { team_id, .. } => {
                        if *team_id == ballot.government.as_ref().map(|t| t.uuid).unwrap_or(Uuid::nil()) {
                            issues.government_issues.push(DrawIssue {target: DrawIssueTarget::Speaker(*non_aligned_id), ..issue});
                        } else if *team_id == ballot.opposition.as_ref().map(|t| t.uuid).unwrap_or(Uuid::nil()) {
                            issues.opposition_issues.push(DrawIssue {target: DrawIssueTarget::Speaker(*non_aligned_id), ..issue});
                        } else {
                            unreachable!()
                        }
                    },
                    _ => unreachable!()
                }
            });
        }

        for gov_speaker_id in ballot.government.iter().flat_map(|t| t.members.iter().map(|m| m.uuid)) {
            let speaker_clashes = self.clash_map.get_clashes_for_participant(&gov_speaker_id);
            ballot.opposition.iter().flat_map(|t| t.members.iter().map(|m| m.uuid)).flat_map(
                |opp_speaker_id| {
                    speaker_clashes.get(&opp_speaker_id).iter().flat_map(|cs| 
                        cs.iter().map(
                            |c| {
                                DrawIssue {
                                    issue_type: c.clash_type.clone(),
                                    severity: (self.get_base_severity(&c.clash_type) as f32 * self.config.team_team_clash_factor) as u16,
                                    target: DrawIssueTarget::Team { team_id: ballot.opposition.as_ref().map(|t| t.uuid).unwrap_or(Uuid::nil()), involved_speakers: vec![opp_speaker_id] }
                                }
                            }
                        ).collect_vec()
                    ).collect_vec()
                }
            ).sorted().coalesce(coalesce_issues).for_each(
                |issue| {
                    issues.government_issues.push(issue.clone());
                    issues.opposition_issues.push(DrawIssue {
                        target: DrawIssueTarget::Team { team_id: ballot.government.as_ref().map(|t| t.uuid).unwrap_or(Uuid::nil()), involved_speakers: vec![gov_speaker_id] },
                        ..issue
                    });
                }
            );
        }

        issues
    }

}

fn coalesce_issues(prev: DrawIssue, next: DrawIssue) -> Result<DrawIssue, (DrawIssue, DrawIssue)> {
    // Some issues should not be repeated individually for each speaker in a team, since that
    // may confuse the user. Specifically a) If we were to account for each insitutional clash
    // individually, we would end up with a lot of clashes for the typical non-mixed team.
    // For the team repetition, we have a similar issue where we would artificially inflate the
    // severity of these clashes.
    match (&prev.issue_type, &next.issue_type) {
        (
            ClashType::JudgeHasSeenTeamSpeaker { round: round_1 },
            ClashType::JudgeHasSeenTeamSpeaker { round: round_2 }
        ) if round_1 == round_2 => {
            Ok(DrawIssue {
                issue_type: ClashType::JudgeHasSeenTeamSpeaker { round: *round_1 },
                severity: u16::max(prev.severity, next.severity),
                target: prev.target.clone()
            })
        },
        (ClashType::InstitutionalClash { severity: severity_1, institution_id: i_id_1 }, ClashType::InstitutionalClash { severity: severity_2, institution_id: i_id_2 }) if i_id_1 == i_id_2 => {
            match (&prev.target, &next.target) {
                (DrawIssueTarget::Team { team_id: t_id_1, involved_speakers: is_1 }, DrawIssueTarget::Team { team_id: t_id_2, involved_speakers: is_2 }) if t_id_1 == t_id_2 => {
                    Ok(DrawIssue {
                        issue_type: ClashType::InstitutionalClash { severity: u16::max(*severity_1, *severity_2), institution_id: *i_id_1 },
                        severity: u16::max(prev.severity, next.severity),
                        target: DrawIssueTarget::Team { team_id: *t_id_1, involved_speakers: is_1.iter().chain(is_2.iter()).copied().collect_vec() }
                    })
                },
                _ => Err((prev, next))
            }
        },
        (_, _) => Err((prev, next))
    }
}



#[cfg(test)]
mod test {
    use itertools::Itertools;
    use sea_orm::prelude::Uuid;

    use crate::{draw::{clashes::{ClashMap, ClashMapEntry, ClashType}, evaluation::{DrawIssue, DrawEvaluatorConfig}}, draw_view::{DrawAdjudicator, DrawBallot, DrawSpeaker, DrawTeam}};

    use super::DrawEvaluator;

    #[test]
    fn test_finds_institution_clashes_between_adjudicators() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(601),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![DrawAdjudicator {uuid: Uuid::from_u128(600), ..Default::default()}, DrawAdjudicator {uuid: Uuid::from_u128(601), ..Default::default()}],
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(clash_map, DrawEvaluatorConfig {
            adj_adj_clash_factor: 2.0,
            ..Default::default()
        });
        let issues = evaluator.find_issues_in_ballot(&ballot);

        assert_eq!(
            issues.adjudicator_issues.get(&Uuid::from_u128(600)).unwrap(),
            &vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Adjudicator(Uuid::from_u128(601)) }]
        );
        assert_eq!(
            issues.adjudicator_issues.get(&Uuid::from_u128(601)).unwrap(),
            &vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Adjudicator(Uuid::from_u128(600)) }]
        );
    }

    #[test]
    fn test_finds_institution_clashes_between_adj_and_non_aligned() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(700),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![DrawAdjudicator {uuid: Uuid::from_u128(600), ..Default::default()}],
            non_aligned_speakers: vec![DrawSpeaker {uuid: Uuid::from_u128(700), ..Default::default()}],
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(clash_map, DrawEvaluatorConfig {
            adj_speaker_clash_factor: 2.0,
            ..Default::default()
        });
        let issues = evaluator.find_issues_in_ballot(&ballot);

        assert_eq!(
            issues.adjudicator_issues.get(&Uuid::from_u128(600)).unwrap(),
            &vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Speaker(Uuid::from_u128(700)) }]
        );
        assert_eq!(
            issues.non_aligned_issues.get(&Uuid::from_u128(700)).unwrap(),
            &vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Adjudicator(Uuid::from_u128(600)) }]
        );
    }

    #[test]
    fn test_finds_institution_clashes_between_adj_and_gov() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(700),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![DrawAdjudicator {uuid: Uuid::from_u128(600), ..Default::default()}],
            government: Some(DrawTeam {
                members: vec![DrawSpeaker {uuid: Uuid::from_u128(700), ..Default::default()}],
                uuid: Uuid::from_u128(800),
                ..Default::default()
            }),
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(clash_map, DrawEvaluatorConfig {
            adj_team_clash_factor: 2.0,
            ..Default::default()
        });
        let issues = evaluator.find_issues_in_ballot(&ballot);

        assert_eq!(
            issues.adjudicator_issues.get(&Uuid::from_u128(600)).unwrap(),
            &vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Team {team_id: Uuid::from_u128(800), involved_speakers: vec![Uuid::from_u128(700)] } }]
        );
        assert_eq!(
            issues.government_issues,
            vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Adjudicator(Uuid::from_u128(600)) }]
        );
    }

    #[test]
    fn test_repeat_institution_clashes_between_adj_and_gov_are_collated() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(700),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );
        clash_map.add_clash_entry(
            Uuid::from_u128(600),
            Uuid::from_u128(701),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 40,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            adjudicators: vec![DrawAdjudicator {uuid: Uuid::from_u128(600), ..Default::default()}],
            government: Some(DrawTeam {
                members: vec![DrawSpeaker {uuid: Uuid::from_u128(700), ..Default::default()}, DrawSpeaker {uuid: Uuid::from_u128(701), ..Default::default()}],
                uuid: Uuid::from_u128(800),
                ..Default::default()
            }),
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(clash_map, DrawEvaluatorConfig {
            adj_team_clash_factor: 2.0,
            ..Default::default()
        });
        let issues = evaluator.find_issues_in_ballot(&ballot);

        let adj_issues = issues.adjudicator_issues.get(&Uuid::from_u128(600)).unwrap();
        assert_eq!(adj_issues[0].severity, 180);
        match adj_issues[0].issue_type {
            ClashType::InstitutionalClash { severity, institution_id } => {
                assert_eq!(severity, 90);
                assert_eq!(institution_id, Uuid::from_u128(100));
                
            },
            _ => panic!("Incorrect Clash typee"),
        }
        match &adj_issues[0].target {
            crate::draw::evaluation::DrawIssueTarget::Team { involved_speakers, team_id } => {
                assert_eq!(involved_speakers.iter().map(|u| *u).sorted().collect_vec(), vec![Uuid::from_u128(700), Uuid::from_u128(701)]);
                assert_eq!(team_id, &Uuid::from_u128(800));
            },
            _ => panic!("Incorrect target type"),
        }

        assert_eq!(
            issues.government_issues,
            vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Adjudicator(Uuid::from_u128(600)) }]
        );
    }

    #[test]
    fn test_finds_institution_clashes_between_gov_and_opp() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(700),
            Uuid::from_u128(710),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            government: Some(DrawTeam {
                members: vec![DrawSpeaker {uuid: Uuid::from_u128(700), ..Default::default()}],
                uuid: Uuid::from_u128(800),
                ..Default::default()
            }),
            opposition: Some(DrawTeam {
                members: vec![DrawSpeaker {uuid: Uuid::from_u128(710), ..Default::default()}],
                uuid: Uuid::from_u128(801),
                ..Default::default()
            }),
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(clash_map, DrawEvaluatorConfig {
            team_team_clash_factor: 2.0,
            ..Default::default()
        });
        let issues = evaluator.find_issues_in_ballot(&ballot);

        assert_eq!(
            issues.government_issues,
            vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Team {team_id: Uuid::from_u128(801), involved_speakers: vec![Uuid::from_u128(710)] } }]
        );
        assert_eq!(
            issues.opposition_issues,
            vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Team {team_id: Uuid::from_u128(800), involved_speakers: vec![Uuid::from_u128(700)] } }]
        );
    }

    #[test]
    fn test_finds_and_collates_institution_clashes_between_opp_and_non_aligned() {
        let mut clash_map = ClashMap::new(Default::default());
        clash_map.add_clash_entry(
            Uuid::from_u128(710),
            Uuid::from_u128(720),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 90,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );
        clash_map.add_clash_entry(
            Uuid::from_u128(720),
            Uuid::from_u128(711),
            ClashMapEntry {
                clash_type: ClashType::InstitutionalClash {
                    severity: 10,
                    institution_id: Uuid::from_u128(100),
                },
            },
        );

        let ballot = DrawBallot {
            opposition: Some(DrawTeam {
                members: vec![DrawSpeaker {uuid: Uuid::from_u128(710), ..Default::default()}, DrawSpeaker {uuid: Uuid::from_u128(711), ..Default::default()}],
                uuid: Uuid::from_u128(801),
                ..Default::default()
            }),
            non_aligned_speakers: vec![DrawSpeaker {uuid: Uuid::from_u128(720), ..Default::default()}],
            ..Default::default()
        };

        let evaluator = DrawEvaluator::new(clash_map, DrawEvaluatorConfig {
            team_speaker_clash_factor: 2.0,
            ..Default::default()
        });
        let issues = evaluator.find_issues_in_ballot(&ballot);

        assert_eq!(
            issues.opposition_issues,
            vec![DrawIssue { issue_type: ClashType::InstitutionalClash { severity: 90, institution_id: Uuid::from_u128(100) }, severity: 180, target: crate::draw::evaluation::DrawIssueTarget::Speaker(Uuid::from_u128(720)) }]
        );
        
        match issues.non_aligned_issues.get(&Uuid::from_u128(720)).unwrap()[0].issue_type {
            ClashType::InstitutionalClash { severity, institution_id } => {
                assert_eq!(severity, 90);
                assert_eq!(institution_id, Uuid::from_u128(100));
                
            },
            _ => panic!("Incorrect Clash typee"),
        }

        match &issues.non_aligned_issues.get(&Uuid::from_u128(720)).unwrap()[0].target {
            crate::draw::evaluation::DrawIssueTarget::Team { involved_speakers, team_id } => {
                assert_eq!(involved_speakers.iter().map(|u| *u).sorted().collect_vec(), vec![Uuid::from_u128(710), Uuid::from_u128(711)]);
                assert_eq!(team_id, &Uuid::from_u128(801));
            },
            _ => panic!("Incorrect target type"),
        }

    }
}
