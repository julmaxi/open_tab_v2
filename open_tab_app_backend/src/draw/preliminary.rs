use std::collections::HashMap;

use open_tab_entities::prelude::{Ballot, BallotTeam, Speech, SpeechRole, TournamentRound};
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, SeedableRng};
use sea_orm::prelude::Uuid;

use thiserror::Error;

use crate::{
    draw_view::{DrawBallot, DrawSpeaker, DrawTeam},
    tab_view::TeamRoundRole,
};

use itertools::Itertools;

use super::{evaluation::DrawEvaluator, optimization::find_best_ballot_assignments};

pub struct RoundGenerationContext {
    pub teams: Vec<DrawTeamInfo>,
    pub speakers: Vec<Uuid>,
    pub adjudicators: Vec<Uuid>,
}

pub struct DrawTeamInfo {
    pub uuid: Uuid,
    pub member_ids: Vec<Uuid>,
}

pub enum PreliminariesDrawMode {
    //Random,
    AvoidClashes,
}


pub struct PreliminaryRoundGenerator {
    pub draw_mode: PreliminariesDrawMode,
    pub randomization_scale: f64,
}

impl Default for PreliminaryRoundGenerator {
    fn default() -> Self {
        PreliminaryRoundGenerator {
            draw_mode: PreliminariesDrawMode::AvoidClashes,
            randomization_scale: 0.5,
        }
    }
}

#[derive(Error, Debug)]
pub enum PreliminaryDrawError {
    #[error("Incorrect number of teams: {0}")]
    IncorrectTeamCount(usize),
    #[error("Incorrect number of rounds: {0}")]
    IncorrectRoundCount(usize),
    #[error("Other error: {source}")]
    Other {
        #[from]
        source: anyhow::Error,
    },
}

impl PreliminaryRoundGenerator {
    pub fn generate_draw_for_rounds(
        &self,
        context: &RoundGenerationContext,
        rounds: Vec<&TournamentRound>,
        evaluator: &DrawEvaluator,
    ) -> Result<Vec<Vec<DrawBallot>>, PreliminaryDrawError> {
        if rounds.len() % 3 != 0 {
            return Err(PreliminaryDrawError::IncorrectRoundCount(rounds.len()));
        }

        if context.teams.len() % 3 != 0 || context.teams.len() == 0 {
            return Err(PreliminaryDrawError::IncorrectTeamCount(
                context.teams.len(),
            ));
        }

        let num_debates = context.teams.len() / 3;

        //let mut rng = thread_rng();
        let mut rng: StdRng = SeedableRng::from_seed([0; 32]);
        let mut shuffled_teams = context.teams.iter().collect::<Vec<_>>();
        shuffled_teams.shuffle(&mut rng);

        let buckets = shuffled_teams
            .chunks(context.teams.len() / 3)
            .map(|v| Vec::from(v))
            .collect::<Vec<_>>();
        let mut role_sequence = [
            TeamRoundRole::Government,
            TeamRoundRole::Opposition,
            TeamRoundRole::NonAligned,
        ];
        role_sequence.shuffle(&mut rng);
        let mut evaluator = evaluator.clone();

        let teams = context
            .teams
            .iter()
            .map(|t| (t.uuid, t.member_ids.clone()))
            .collect::<HashMap<_, _>>();

        let round_ballots: Result<Vec<Vec<DrawBallot>>, _> = rounds
            .iter()
            .enumerate()
            .map(|(round_idx, round)| {
                let ballots = (0..num_debates)
                    .map(|_| DrawBallot {
                        ..Default::default()
                    })
                    .collect::<Vec<_>>();

                let bucket_1 = &buckets[0];
                let bucket_2 = &buckets[1];
                let bucket_3 = &buckets[2];

                let bucket_1_role = &role_sequence[round_idx % 3];
                let bucket_2_role = &role_sequence[(round_idx + 1) % 3];
                let bucket_3_role = &role_sequence[(round_idx + 2) % 3];

                let (gov_bucket, opp_bucket, non_aligned_bucket) =
                    match (bucket_1_role, bucket_2_role, bucket_3_role) {
                        (
                            TeamRoundRole::Government,
                            TeamRoundRole::Opposition,
                            TeamRoundRole::NonAligned,
                        ) => (bucket_1, bucket_2, bucket_3),
                        (
                            TeamRoundRole::Government,
                            TeamRoundRole::NonAligned,
                            TeamRoundRole::Opposition,
                        ) => (bucket_1, bucket_3, bucket_2),
                        (
                            TeamRoundRole::Opposition,
                            TeamRoundRole::Government,
                            TeamRoundRole::NonAligned,
                        ) => (bucket_2, bucket_1, bucket_3),
                        (
                            TeamRoundRole::Opposition,
                            TeamRoundRole::NonAligned,
                            TeamRoundRole::Government,
                        ) => (bucket_2, bucket_3, bucket_1),
                        (
                            TeamRoundRole::NonAligned,
                            TeamRoundRole::Government,
                            TeamRoundRole::Opposition,
                        ) => (bucket_3, bucket_1, bucket_2),
                        (
                            TeamRoundRole::NonAligned,
                            TeamRoundRole::Opposition,
                            TeamRoundRole::Government,
                        ) => (bucket_3, bucket_2, bucket_1),
                        _ => unreachable!(),
                    };

                let ballots = self.assign_teams_to_ballots(
                    &ballots,
                    gov_bucket,
                    opp_bucket,
                    non_aligned_bucket,
                    &evaluator,
                );

                if let Ok(ballots) = ballots.as_ref() {
                    let draw_ballots = ballots
                        .iter()
                        .map(|b| Ballot {
                            uuid: Uuid::nil(),
                            speeches: b
                                .non_aligned_speakers
                                .iter()
                                .enumerate()
                                .map(|(idx, s)| Speech {
                                    speaker: Some(s.uuid),
                                    role: SpeechRole::NonAligned,
                                    is_opt_out: false,
                                    position: idx as u8,
                                    scores: HashMap::new(),
                                })
                                .collect(),
                            government: BallotTeam {
                                team: b.government.as_ref().map(|t| t.uuid),
                                scores: HashMap::new(),
                            },
                            opposition: BallotTeam {
                                team: b.opposition.as_ref().map(|t| t.uuid),
                                scores: HashMap::new(),
                            },
                            adjudicators: vec![],
                            president: None,
                        })
                        .collect::<Vec<Ballot>>();

                    evaluator.clash_map.add_dynamic_clashes_from_round_ballots(
                        vec![&(round.uuid, draw_ballots)],
                        &teams,
                    )?;
                }
                ballots
            })
            .collect();

        Ok(round_ballots?)
    }

    fn assign_teams_to_ballots(
        &self,
        ballots: &Vec<DrawBallot>,
        gov_bucket: &Vec<&DrawTeamInfo>,
        opp_bucket: &Vec<&DrawTeamInfo>,
        non_aligned_bucket: &Vec<&DrawTeamInfo>,
        evaluator: &DrawEvaluator,
    ) -> Result<Vec<DrawBallot>, anyhow::Error> {
        let mut out_ballots = ballots.clone();
        let mut rng = thread_rng();

        let mut non_aligned_bucket_position_buckets = (0..3).map(|_| Vec::new()).collect_vec();

        non_aligned_bucket.iter().for_each(|team| {
            let mut member_ids = team.member_ids.clone();
            member_ids.shuffle(&mut rng);

            for i in 0..3 {
                non_aligned_bucket_position_buckets[i].push(member_ids[i]);
            }
        });

        let mut gov_bucket = gov_bucket.clone();
        gov_bucket.shuffle(&mut rng);
        for (ballot_idx, ballot) in out_ballots.iter_mut().enumerate() {
            ballot.government = Some(DrawTeam {
                uuid: gov_bucket[ballot_idx].uuid,
                ..Default::default()
            });
        }

        match self.draw_mode {
            PreliminariesDrawMode::AvoidClashes => {
                let possible_ballots = opp_bucket
                    .iter()
                    .map(|team| {
                        out_ballots
                            .iter()
                            .map(|ballot| DrawBallot {
                                opposition: Some(DrawTeam {
                                    uuid: team.uuid,
                                    ..Default::default()
                                }),
                                ..ballot.clone()
                            })
                            .collect_vec()
                    })
                    .collect_vec();

                out_ballots = find_best_ballot_assignments(
                    &possible_ballots,
                    evaluator,
                    self.randomization_scale,
                )?;

                for non_aligned_position in 0..3 {
                    let possible_ballots = non_aligned_bucket_position_buckets
                        [non_aligned_position]
                        .iter()
                        .map(|speaker_id| {
                            out_ballots
                                .iter()
                                .map(|ballot| {
                                    let mut new_non_aligned = ballot.non_aligned_speakers.clone();
                                    new_non_aligned.push(DrawSpeaker {
                                        uuid: speaker_id.clone(),
                                        ..Default::default()
                                    });
                                    DrawBallot {
                                        non_aligned_speakers: new_non_aligned,
                                        ..ballot.clone()
                                    }
                                })
                                .collect_vec()
                        })
                        .collect_vec();

                    out_ballots = find_best_ballot_assignments(
                        &possible_ballots,
                        evaluator,
                        self.randomization_scale,
                    )?;
                }
            }
        };

        Ok(out_ballots)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use itertools::Itertools;
    use open_tab_entities::prelude::TournamentRound;
    use sea_orm::prelude::Uuid;

    use crate::{
        draw::{
            clashes::{ClashMap, ClashMapEntry, ClashType},
            evaluation::DrawEvaluator,
        },
        draw_view::DrawBallot,
    };

    use super::{DrawTeamInfo, PreliminaryRoundGenerator, RoundGenerationContext};

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum TeamPositionStatisticEntry {
        Government,
        Opposition,
        NonAligned { rooms: Vec<usize> },
        None,
    }

    fn compute_team_position_history(
        round_ballots: &Vec<Vec<DrawBallot>>,
        teams: Vec<DrawTeamInfo>,
    ) -> HashMap<Uuid, Vec<TeamPositionStatisticEntry>> {
        let mut team_stats = HashMap::new();

        let member_team_id_map = teams
            .iter()
            .flat_map(|team| {
                team.member_ids
                    .iter()
                    .map(|member_id| (member_id, team.uuid.clone()))
            })
            .collect::<HashMap<_, _>>();

        for (round_idx, this_round_ballots) in round_ballots.iter().enumerate() {
            for (ballot_idx, ballot) in this_round_ballots.iter().enumerate() {
                if let Some(government) = ballot.government.as_ref() {
                    let gov_stats: &mut Vec<TeamPositionStatisticEntry> = team_stats
                        .entry(government.uuid)
                        .or_insert_with(|| {
                            (0..round_ballots.len())
                                .map(|_| TeamPositionStatisticEntry::None)
                                .collect_vec()
                        })
                        .as_mut();
                    assert!(gov_stats[round_idx] == TeamPositionStatisticEntry::None);
                    gov_stats[round_idx] = TeamPositionStatisticEntry::Government;
                }
                if let Some(opposition) = ballot.opposition.as_ref() {
                    let opp_stats: &mut Vec<TeamPositionStatisticEntry> = team_stats
                        .entry(opposition.uuid)
                        .or_insert_with(|| {
                            (0..round_ballots.len())
                                .map(|_| TeamPositionStatisticEntry::None)
                                .collect_vec()
                        })
                        .as_mut();
                    assert!(opp_stats[round_idx] == TeamPositionStatisticEntry::None);
                    opp_stats[round_idx] = TeamPositionStatisticEntry::Opposition;
                }

                for non_aligned in &ballot.non_aligned_speakers {
                    let team_id = *member_team_id_map.get(&non_aligned.uuid).unwrap();
                    let non_aligned_stats: &mut Vec<TeamPositionStatisticEntry> = team_stats
                        .entry(team_id)
                        .or_insert_with(|| {
                            (0..round_ballots.len())
                                .map(|_| TeamPositionStatisticEntry::None)
                                .collect_vec()
                        })
                        .as_mut();
                    match &mut non_aligned_stats[round_idx] {
                        TeamPositionStatisticEntry::NonAligned { rooms } => rooms.push(ballot_idx),
                        TeamPositionStatisticEntry::None => {
                            non_aligned_stats[round_idx] = TeamPositionStatisticEntry::NonAligned {
                                rooms: vec![ballot_idx],
                            }
                        }
                        _ => panic!("Team has mixed role"),
                    }
                }
            }
        }

        team_stats
    }

    #[test]
    fn test_generated_draw_has_correct_statistics() -> Result<(), anyhow::Error> {
        let context = RoundGenerationContext {
            teams: (0..12)
                .map(|idx| {
                    let base_id = 4000 + idx * 10;
                    DrawTeamInfo {
                        uuid: Uuid::from_u128(base_id),
                        member_ids: vec![
                            Uuid::from_u128(base_id + 1),
                            Uuid::from_u128(base_id + 2),
                            Uuid::from_u128(base_id + 2),
                        ],
                    }
                })
                .collect(),
            adjudicators: vec![],
            speakers: vec![]
        };

        let generator = PreliminaryRoundGenerator {
            ..Default::default()
        };

        let rounds = vec![
            TournamentRound {
                uuid: Uuid::from_u128(10),
                tournament_id: Uuid::from_u128(0),
                index: 0,
                ..Default::default()
            },
            TournamentRound {
                uuid: Uuid::from_u128(11),
                tournament_id: Uuid::from_u128(0),
                index: 0,
                ..Default::default()
            },
            TournamentRound {
                uuid: Uuid::from_u128(12),
                tournament_id: Uuid::from_u128(0),
                index: 0,
                ..Default::default()
            },
        ];

        let mut clash_map = ClashMap::new(Default::default());

        for idx in 0..12 {
            let base_id = 4000 + idx * 10;

            clash_map.add_clash_entry(
                Uuid::from_u128(base_id + 1),
                Uuid::from_u128(base_id + 2),
                ClashMapEntry {
                    clash_type: ClashType::SameTeamClash,
                },
            )
        }

        let evaluator = DrawEvaluator::new(clash_map, Default::default(), HashMap::new());
        let generated_ballots =
            generator.generate_draw_for_rounds(&context, rounds.iter().collect(), &evaluator)?;
        let stats = compute_team_position_history(&generated_ballots, context.teams);

        assert_eq!(stats.len(), 12, "All teams should have a statistics entry");

        for team_stats in stats.values() {
            let positions = team_stats
                .iter()
                .map(|e| std::mem::discriminant(e))
                .collect_vec();

            assert!(
                !positions.contains(&std::mem::discriminant(&TeamPositionStatisticEntry::None)),
                "Team must always be set"
            );
            assert_eq!(
                positions.iter().unique().map(|d| *d).collect_vec().len(),
                3,
                "Team must see all three roles"
            );
        }

        Ok(())
    }
}
