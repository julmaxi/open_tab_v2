use std::{sync::Arc, collections::{HashSet, HashMap}, cmp::Ordering};

use itertools::{Itertools, izip, repeat_n};
use async_trait::async_trait;
use open_tab_entities::{derived_models::{BackupBallot, BreakNodeBackgroundInfo, NodeExecutionError}, domain::{entity::LoadEntity, tournament_break::TournamentBreak, tournament_plan_edge::TournamentPlanEdge, tournament_plan_node::{BreakConfig, PlanNodeType, RoundGroupConfig, TournamentPlanNode}, tournament_venue::TournamentVenue}, prelude::*, schema::speaker, tab::TeamRoundRole, EntityTypeId};
use open_tab_entities::domain::tournament_plan_node::TournamentEligibleBreakCategory;

use rand::{thread_rng, Rng};
use sea_orm::prelude::*;

use crate::{draw::{evaluation::{DrawConstructionEvaluationContext, DrawEvaluator, DrawEvaluatorConfig}, flow_optimization::{OptimizationOptions, OptimizationState}, preliminary::{DrawTeamInfo, RoundGenerationContext}, tab_draw::{assign_teams, pair_speakers, pair_teams, TeamPair}, PreliminariesDrawMode, PreliminaryRoundGenerator}, draw_view::{DrawAdjudicator, DrawBallot, DrawSpeaker, DrawTeam, SetDrawAdjudicator}, views, TournamentParticipantsInfo};
use serde::{Serialize, Deserialize};

use super::{ActionTrait, edit_tree::reindex_rounds};

use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutePlanNodeAction {
    pub tournament_id: Uuid,
    pub plan_node: Uuid
}

fn round_draw_from_team_and_speaker_pairs(team_pairs: Vec<TeamPair>, speaker_pairs: Vec<Vec<Uuid>>) -> Vec<DrawBallot> {
    let (team_pairs, speaker_pairs) = if team_pairs.len() <= speaker_pairs.len() {
        (team_pairs, speaker_pairs)
    }
    else {
        let n_speaker_pairs = speaker_pairs.len();
        let new_speaker_pairs = speaker_pairs.into_iter().chain(
            repeat_n(vec![], team_pairs.len() - n_speaker_pairs)
        ).collect_vec();
        (team_pairs, new_speaker_pairs)
    };

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
                    |speaker_id| Some(DrawSpeaker {
                        uuid: speaker_id,
                        ..Default::default()
                    })
                ).collect(),
                ..Default::default()
            };
            ballot
        }
    ).collect_vec()
}


//Ignores conflict and non-aligned
fn get_gov_opp_assignments_from_ballots(ballots: &Vec<Ballot>) -> HashMap<Uuid, TeamRoundRole> {
    ballots.iter().flat_map(
        |b| {
            let mut out = vec![];
            if let Some(gov) = b.government.team {
                out.push((gov, TeamRoundRole::Government));
            }
            if let Some(opp) = b.opposition.team {
                out.push((opp, TeamRoundRole::Opposition));
            }
            out
        }
    ).collect()

}


async fn generate_round_draw<C>(db: &C, tournament_id: Uuid, node_id: Uuid, config: &RoundGroupConfig, existing_rounds: &Vec<Uuid>) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
    let mut changes = EntityGroup::new(tournament_id);

    let all_nodes = TournamentPlanNode::get_all_in_tournament(db, tournament_id).await?;
    let edges = TournamentPlanEdge::get_all_for_sources(db, all_nodes.iter().map(|n| n.uuid).collect()).await?;

    
    let mut all_nodes = all_nodes.into_iter().map(|n| (n.uuid, n)).collect::<HashMap<_, _>>();
    let parent_map = edges.into_iter().map(|e| (e.target_id, e.source_id)).collect::<HashMap<_, _>>();

    let mut relevant_break_id = None;
    let mut immediately_preceding_round_id = None;
    let mut curr_node_id_option = parent_map.get(&node_id).cloned();
    while let Some(curr_node_id) = curr_node_id_option {
        let node: &TournamentPlanNode = all_nodes.get(&curr_node_id).ok_or(NodeExecutionError::RoundIsNotInTournament { tournament_id })?;
        match &node.config {
            PlanNodeType::Break { config: _, break_id, .. } => {
                if relevant_break_id.is_none() {
                    relevant_break_id = Some(break_id.clone());

                    if immediately_preceding_round_id.is_some() {
                        break;
                    }
                }
            },
            PlanNodeType::Round { config: _, rounds } => {
                if immediately_preceding_round_id.is_none() && rounds.len() > 0 {
                    immediately_preceding_round_id = Some(rounds.last().unwrap().clone());

                    if relevant_break_id.is_some() {
                        break;
                    }
                }
            },
        }

        let parent = parent_map.get(&curr_node_id);
        curr_node_id_option = parent.cloned();
    };

    let all_teams = Team::get_all_in_tournament(db, tournament_id).await?;
    let all_participants = Participant::get_all_in_tournament(db, tournament_id).await?;

    let team_members = all_participants.iter().filter_map(
        |p| match &p.role {
            ParticipantRole::Speaker(s) if s.team_id.is_some() => Some((s.team_id.unwrap(), p.uuid)),
            _ => None
        }
    ).into_group_map();

    let (all_teams, all_speakers, all_adjudicator_ids) = if let Some(relevant_break_id) = relevant_break_id {
        if let Some(relevant_break_id) = relevant_break_id {
            let break_ = TournamentBreak::get(db, relevant_break_id).await?;
            let teams = break_.breaking_teams;
            let speakers = break_.breaking_speakers;

            let teams_by_id = all_teams.into_iter().map(|t| (t.uuid, t)).collect::<HashMap<_, _>>();
            let speakers_by_id = all_participants.into_iter().map(
                |p| (p.uuid, p)
            ).collect::<HashMap<_, _>>();
            
            (
                teams.into_iter().map(|t| teams_by_id.get(&t).cloned().expect("Guaranteed by db constraint")).collect_vec(),
                speakers.into_iter().map(|s| speakers_by_id.get(&s).cloned().expect("Guaranteed by db constraint")).collect_vec(),
                break_.breaking_adjudicators
            )
        }
        else {
            return Err(NodeExecutionError::MissingBreak.into());
        }
    }
    else {
        (
            all_teams,
            all_participants,
            vec![]
        )
    };

    let other_rounds = TournamentRound::get_all_in_tournament(db, tournament_id).await?.into_iter().filter(
        |r| !existing_rounds.contains(&r.uuid)
    ).collect_vec();

    let mut evaluation_context = DrawConstructionEvaluationContext::new_from_tournament(db, tournament_id).await?;

    let context = RoundGenerationContext {
        teams: all_teams.iter().map(|t| DrawTeamInfo {
            uuid: t.uuid,
            member_ids: team_members.get(&t.uuid).cloned().unwrap_or_default()
        }).collect(),
        adjudicators: all_speakers.iter().filter_map(
            |a| match a.role {
                ParticipantRole::Adjudicator(_) => Some(a.uuid),
                _ => None
            }
        ).collect(),
        speakers: all_speakers.iter().filter_map(
            |a| match a.role {
                ParticipantRole::Speaker(_) => Some(a.uuid),
                _ => None
            }
        ).collect(),
    };

    let mut rounds = TournamentRound::get_many(db, existing_rounds.clone()).await?;

    if rounds.len() > config.num_rounds() as usize {
        for round in rounds.drain((config.num_rounds() as usize)..) {
            changes.delete(EntityTypeId::TournamentRound, round.uuid);
        }
    }
    else if rounds.len() < config.num_rounds() as usize {
        for _ in rounds.len()..(config.num_rounds() as usize) {
            //This indexing is used to make sure these rounds always get
            //reindex and thus saved.
            let round: TournamentRound = TournamentRound::new(tournament_id, u64::MAX);
            rounds.push(round);
        }
    }

    let mut original_node = all_nodes.get(&node_id).expect("Guaranteed by db constraints").clone();
    original_node.config = PlanNodeType::Round { config: config.clone(), rounds: rounds.iter().map(|r| r.uuid).collect() };
    all_nodes.insert(node_id, original_node);

    let ballots = match config {
        RoundGroupConfig::Preliminaries { num_roundtrips: _ } => {
            let generator = PreliminaryRoundGenerator {
                draw_mode: PreliminariesDrawMode::AvoidClashes,
                randomization_scale: 0.5
            };

            let ballots = generator.generate_draw_for_rounds(
                &context,
                rounds.iter().collect(),
                other_rounds.iter().map(|r| r.uuid).collect(),
                &mut evaluation_context
            )?;
            ballots
        },
        RoundGroupConfig::FoldDraw { round_configs } => {
            let team_pairs = round_configs.iter().map(|c| pair_teams(&all_teams.iter().map(|t| t.uuid).collect_vec(), &c.team_fold_method)).collect_vec();
            
            let mut preceding_round_gov_opp_assignments = if let Some(immediately_preceding_round_id) = immediately_preceding_round_id {
                let (_, round_ballots) = Ballot::get_all_in_rounds(db, vec![immediately_preceding_round_id]).await?.into_iter().next().expect("Round existence guaranteed by db constraints.");

                Some(get_gov_opp_assignments_from_ballots(&round_ballots))
            }
            else {
                None
            };

            let speakers = all_speakers.into_iter().map(|s| s.uuid).collect_vec();

            let team_and_speaker_pairs = izip!(team_pairs.into_iter(), round_configs.iter()).map(|(team_pairs, config)| {
                let team_pairs = assign_teams(team_pairs, config, preceding_round_gov_opp_assignments.as_ref());
                let speaker_pairs = pair_speakers(&speakers, &config.non_aligned_fold_method);

                if preceding_round_gov_opp_assignments.is_some() {
                    preceding_round_gov_opp_assignments.as_mut().unwrap().clear();
                }
                else {
                    let _ = preceding_round_gov_opp_assignments.insert(HashMap::new());
                };
                for pair in team_pairs.iter() {
                    let gov_opp_dict = preceding_round_gov_opp_assignments.as_mut().unwrap();
                    gov_opp_dict.insert(pair.government_id, TeamRoundRole::Government);
                    gov_opp_dict.insert(pair.opposition_id, TeamRoundRole::Opposition);
                }
                (team_pairs, speaker_pairs)
            }).collect_vec();

            team_and_speaker_pairs.into_iter().map(|(team_pairs, speaker_pairs)| round_draw_from_team_and_speaker_pairs(team_pairs, speaker_pairs)).collect_vec()
        },
    };

    let mut optimization_state = OptimizationState::load_from_rounds_and_draw_ballots(db, tournament_id, rounds.iter().zip(ballots.iter()).collect(), OptimizationOptions::default()).await?;

    let adjudicators_to_include = if all_adjudicator_ids.len() > 0 {
        Some(&all_adjudicator_ids)
    }
    else {
        None
    };
    let evaluator = DrawEvaluator::new(DrawEvaluatorConfig::default(), other_rounds.iter().map(|r| r.uuid).collect(), &evaluation_context);
    optimization_state.update_state_by_assigning_adjudicators(
        adjudicators_to_include,
        &evaluator
    );

    let ballots = optimization_state.rounds.iter().zip(ballots).map(|(r, ballots)| {
        r.debates.iter().zip(ballots.iter()).map(
            |(debate, ballot)| {
                let mut ballot = ballot.clone();
                ballot.adjudicators = debate.chair.iter().chain(debate.wings.iter()).map(
                    |debate_adjudicator| SetDrawAdjudicator{
                        adjudicator: DrawAdjudicator {
                        uuid: *debate_adjudicator,
                        ..Default::default()
                    }, ..Default::default()
                    }
                ).collect();
                ballot
            }
        ).collect_vec()
    }).collect_vec();

    let existing_debates = TournamentDebate::get_all_in_rounds(db, rounds.iter().map(|r| r.uuid).collect()).await?;
    let mut all_venues = TournamentVenue::get_all_in_tournament(db, tournament_id).await?;
    all_venues.sort_by_key(|v| v.ordering_index);

    for (round, round_existing_debates, round_new_ballots) in izip![rounds.iter(), existing_debates.into_iter(), ballots.into_iter()] {
        let debates = if round_existing_debates.len() < round_new_ballots.len() {
            let new_debates = (round_existing_debates.len()..round_new_ballots.len()).map(
                |index| TournamentDebate::new(round.uuid, index as u64, Uuid::nil(), None)
            );
            round_existing_debates.into_iter().chain(
                new_debates
            ).collect_vec()
        }
        else if round_existing_debates.len() > round_new_ballots.len() {
            for debate in round_existing_debates.iter().skip(round_new_ballots.len()) {
                changes.delete(
                    EntityTypeId::TournamentDebate,
                    debate.uuid
                );

                changes.delete(
                    EntityTypeId::Ballot,
                    debate.ballot_id
                );

                let backup_ballots = open_tab_entities::domain::debate_backup_ballot::DebateBackupBallot::get_all_for_debate(db, debate.uuid).await?;

                for backup_ballot in backup_ballots {
                    changes.delete(
                        EntityTypeId::DebateBackupBallot,
                        backup_ballot.uuid
                    );
                }
            }

            round_existing_debates.into_iter().take(round_new_ballots.len()).collect_vec()
        }
        else {
            round_existing_debates
        };

        // Preserve any previously assigned venues
        let used_venues = debates.iter().filter_map(|d| d.venue_id).collect::<HashSet<_>>();
        let available_venues = all_venues.iter().filter(|v| !used_venues.contains(&v.uuid)).collect::<Vec<_>>();

        let selected_venues = if available_venues.len() >= debates.iter().filter(|d| d.venue_id.is_none()).count() {
            available_venues.iter().map(|v| Some(v.uuid)).collect::<Vec<_>>()
        } else {
            itertools::repeat_n(None, debates.len()).collect::<Vec<_>>()
        };
        let mut selected_venues = selected_venues.iter();

        for (mut debate, ballot) in izip!(debates.into_iter(), round_new_ballots.into_iter()) {
            let mut real_ballot : Ballot = ballot.into();
            if debate.ballot_id.is_nil() {
                debate.ballot_id = Uuid::new_v4();
            }
            real_ballot.uuid = debate.ballot_id;
            //real_ballot.uuid = Uuid::new_v4();
            //debate.ballot_id = real_ballot.uuid;

            if debate.venue_id.is_none() {
                selected_venues.next().map(|v| debate.venue_id = *v);
            }

            changes.add(Entity::Ballot(real_ballot));
            changes.add(Entity::TournamentDebate(debate));
        }
    }

    let all_edges = TournamentPlanEdge::get_all_for_sources(db, all_nodes.iter().map(|n| *n.0).collect()).await?;

    let all_rounds = itertools::chain(rounds, other_rounds).collect_vec();

    changes.add(Entity::TournamentPlanNode(all_nodes.get(&node_id).expect("Guaranteed by db constraints").clone()));

    reindex_rounds(&all_nodes.into_values().collect(), &all_edges, &all_rounds).into_iter().for_each(
        |r| {
            changes.add(Entity::TournamentRound(r))
        }
    );
    Ok(changes)
}


//pub async fn 

//async fn generate_fC>(db: &C, tournament_id: Uuid, node_id: Uuid, config: &RoundGroupConfig) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {

fn find_speakers_not_in_teams(
    teams: &Vec<Uuid>,
    speaker_ranking: &Vec<Uuid>,
    team_members: &HashMap<Uuid, Vec<Uuid>>,
) -> Vec<Uuid> {
    let team_breaking_ids = teams.iter().map(|t|
        team_members.get(t).clone().into_iter().flatten()
    ).flatten().collect_vec();
    speaker_ranking.iter()
    .filter(
        |e| {
            !team_breaking_ids.contains(&e)
        }
    ).map(|s| *s).collect_vec()
}

#[derive(Error, Debug)]
pub enum MakeBreakError {
    #[error("KO breaks require a single 'ko' round in dependency")]
    KOBreakConditionNotMet,
    #[error("KO breaks require drawn and scored round")]
    KORoundIncompleteRound,
    #[error("Break require enough teams")]
    NotEnoughTeams,
    #[error("Invalid team count")]
    InvalidTeamCount,
    #[error("Manual breaks can not be automatically computed")]
    IsManualBreak,
}

pub struct EligibilityInfo {
    pub eligible_teams: HashSet<Uuid>,
    pub eligible_speakers: HashSet<Uuid>,
    pub eligible_adjudicators: HashSet<Uuid>,
}

pub fn get_eligiblity_info(
    eligible_categories: &Vec<TournamentEligibleBreakCategory>,
    participants: &HashMap<Uuid, Participant>,
) -> EligibilityInfo {
    let team_members = participants.values().filter_map(|p| {
        match p.role {
            ParticipantRole::Speaker(Speaker {team_id: Some(team_id), ..}) => Some((team_id, p.uuid)),
            _ => None
        }
    }).into_group_map();

    let speakers = participants.values().filter_map(
        |p| {
            match p.role {
                ParticipantRole::Speaker(_) => Some(p.uuid),
                _ => None
            }
        }
    ).collect();

    let adjudicators = participants.values().filter_map(
        |p| {
            match p.role {
                ParticipantRole::Adjudicator(_) => Some(p.uuid),
                _ => None
            }
        }
    ).collect();

    if eligible_categories.len() == 0 {
        return EligibilityInfo {
            eligible_teams: team_members.keys().cloned().collect(),
            eligible_speakers: speakers,
            eligible_adjudicators: adjudicators,
        };
    }

    let mut category_qualified_teams = eligible_categories.iter().map(
        |c| {
            let eligible_teams = if c.config.team_eligibility_mode != open_tab_entities::domain::tournament_plan_node::TeamEligibilityMode::DoNotRestrict {
                let eligible_teams = team_members.iter().filter_map(
                    |(team_id, members)| {
                        let num_qualified_members = members.iter().filter(
                            |m| {
                                participants.get(m).map(
                                    |p| p.break_category_id
                                ).flatten() == Some(c.category_id)
                            }
                        ).count();

                        let is_eligible = match c.config.team_eligibility_mode {
                            open_tab_entities::domain::tournament_plan_node::TeamEligibilityMode::AnyEligible => {
                                num_qualified_members > 0
                            },
                            open_tab_entities::domain::tournament_plan_node::TeamEligibilityMode::MajorityEligible => {
                                num_qualified_members >= ((members.len() as f32) / 2.0).ceil() as usize
                            },
                            open_tab_entities::domain::tournament_plan_node::TeamEligibilityMode::AllEligible => {
                                num_qualified_members == members.len()
                            },
                            open_tab_entities::domain::tournament_plan_node::TeamEligibilityMode::DoNotRestrict => {
                                unreachable!()
                            }
                        };

                        if is_eligible {
                            Some(*team_id)
                        }
                        else {
                            None
                        }
                    }
                ).collect_vec();
                Some(eligible_teams)
            }
            else {
                None
            };

            (c.category_id, eligible_teams)
        }
    ).collect::<HashMap<_, _>>();

    let mut category_qualified_speakers = eligible_categories.iter().map(
        |c| {
            let eligible_participants = if c.config.non_aligned_eligibility_mode != open_tab_entities::domain::tournament_plan_node::NonAlignedEligibilityMode::DoNotRestrict {
                let eligible_participants = participants.values().filter_map(
                    |p| {
                            match p.role {
                                ParticipantRole::Speaker(Speaker {team_id, ..}) => {
                                    let is_natively_eligible = p.break_category_id == Some(c.category_id);

                                    let is_team_elible = team_id.map(
                                        |t| {
                                            let teams = category_qualified_teams.get(&c.category_id).unwrap();
                                            
                                            if let Some(teams) = teams {
                                                teams.contains(&t)
                                            }
                                            else {
                                                true
                                            }
                                        }
                                    ).unwrap_or(false);

                                    let is_eligible = match c.config.non_aligned_eligibility_mode {
                                        open_tab_entities::domain::tournament_plan_node::NonAlignedEligibilityMode::AllEligible => is_natively_eligible,
                                        open_tab_entities::domain::tournament_plan_node::NonAlignedEligibilityMode::AllInEligibleTeams => is_team_elible,
                                        open_tab_entities::domain::tournament_plan_node::NonAlignedEligibilityMode::AllEligibleInEligibleTeams => is_natively_eligible && is_team_elible,
                                        open_tab_entities::domain::tournament_plan_node::NonAlignedEligibilityMode::DoNotRestrict => unreachable!(),
                                    };

                                    if is_eligible {
                                        Some(p.uuid)
                                    }
                                    else {
                                        None
                                    }
                                },
                                _ => None
                            }
                    }
                ).collect_vec();
                Some(eligible_participants)
            }
            else {
                None
            };

            (c.category_id, eligible_participants)
        }
    ).collect::<HashMap<_, _>>();

    let mut qualified_adjudicators = eligible_categories.iter().map(
        |c| {
            let eligible_adjudicators = if c.config.adjudicator_eligibility_mode != open_tab_entities::domain::tournament_plan_node::AdjudicatorEligibilityMode::DoNotRestrict {
                let eligible_adjudicators = participants.values().filter_map(
                    |p| {
                        match p.role {
                            ParticipantRole::Adjudicator(_) => {
                                let is_natively_eligible = p.break_category_id == Some(c.category_id);

                                let is_eligible = match c.config.adjudicator_eligibility_mode {
                                    open_tab_entities::domain::tournament_plan_node::AdjudicatorEligibilityMode::DoNotRestrict => unreachable!(),
                                    open_tab_entities::domain::tournament_plan_node::AdjudicatorEligibilityMode::AllEligible => is_natively_eligible,
                                };

                                if is_eligible {
                                    Some(p.uuid)
                                }
                                else {
                                    None
                                }
                            },
                            _ => None
                        }
                    }
                ).collect_vec();
                Some(eligible_adjudicators)
            }
            else {
                None
            };

            (c.category_id, eligible_adjudicators)
        }
    ).collect::<HashMap<_, _>>();

    let mut out_info = EligibilityInfo {
        eligible_teams: HashSet::new(),
        eligible_speakers: HashSet::new(),
        eligible_adjudicators: HashSet::new(),
    };

    let mut did_restrict_teams = false;
    let mut did_restrict_speakers = false;
    let mut did_restrict_adjudicators = false;

    for category in eligible_categories {
        let team_ids = category_qualified_teams.remove(&category.category_id).flatten();
        let speaker_ids = category_qualified_speakers.remove(&category.category_id).flatten();
        let adjudicator_ids = qualified_adjudicators.remove(&category.category_id).flatten();

        if let Some(team_ids) = team_ids {
            out_info.eligible_teams.extend(team_ids.iter().cloned());
            did_restrict_teams = true;
        }
        if let Some(eligible_speakers) = speaker_ids {
            out_info.eligible_speakers.extend(eligible_speakers.iter().cloned());
            did_restrict_speakers = true;
        }
        if let Some(adjudicator_ids) = adjudicator_ids {
            out_info.eligible_adjudicators.extend(adjudicator_ids.iter().cloned());
            did_restrict_adjudicators = true;
        }
    }

    if !did_restrict_teams {
        out_info.eligible_teams = team_members.keys().cloned().collect();
    }

    if !did_restrict_speakers {
        out_info.eligible_speakers = speakers;
    }

    if !did_restrict_adjudicators {
        out_info.eligible_adjudicators = adjudicators;
    }

    out_info
}


async fn generate_break<C>(db: &C, tournament_id: Uuid, node_id: Uuid, config: &BreakConfig, break_id: Option<Uuid>, eligible_categories: &Vec<TournamentEligibleBreakCategory>, suggested_award_title: &Option<String>, suggested_break_award_prestige: &Option<i32>, participants: &HashMap<Uuid, Participant>) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
    let mut groups = EntityGroup::new(tournament_id);

    let break_background = BreakNodeBackgroundInfo::load_for_break_node(db, tournament_id, node_id).await?;

    let all_nodes = break_background.all_nodes;
    let preceding_rounds = break_background.preceding_rounds;
    let _relevant_break = break_background.relevant_break_id;

    let speaker_info = TournamentParticipantsInfo::load(db, tournament_id).await?;

    let tab = views::tab_view::TabView::load_from_rounds(
        db,
        preceding_rounds.clone(),
        &speaker_info.team_members
    ).await?;

    let eligibility_info = get_eligiblity_info(
        eligible_categories,
        participants
    );

    let team_ranking = tab.team_tab.iter()
        .filter(
            |t| {
                eligibility_info.eligible_teams.contains(&t.team_uuid)
            }
        )
        .map(|t| (ordered_float::NotNan::new(t.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap(), t))
        .sorted_by_key(|t| t.0)
        .rev().map(|t| t.1.team_uuid).collect_vec();

    let speaker_ranking = tab.speaker_tab.iter()
        .filter(
            |s| {
                eligibility_info.eligible_speakers.contains(&s.speaker_uuid)
            }
        )
        .map(|s| (ordered_float::NotNan::new(s.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap(), s))
        .sorted_by_key(|s| s.0 )
        .rev()
        .map(|s| s.1.speaker_uuid).collect_vec();

    let mut break_ = TournamentBreak::new(tournament_id);

    match config {
        open_tab_entities::domain::tournament_plan_node::BreakConfig::TabBreak { num_teams, num_non_aligned } => {
            let teams = team_ranking.into_iter().take((*num_teams) as usize).collect_vec();
            if teams.len() < *num_teams as usize {
                return Err(MakeBreakError::NotEnoughTeams.into());
            }
            let speakers = find_speakers_not_in_teams(&teams, &speaker_ranking, &speaker_info.team_members);
            let speakers = speakers.into_iter().take((*num_non_aligned) as usize).collect_vec();

            break_.breaking_teams = teams;
            break_.breaking_speakers = speakers;
        },
        open_tab_entities::domain::tournament_plan_node::BreakConfig::TwoThirdsBreak => {
            if team_ranking.len() < 3 || team_ranking.len() % 3 != 0 {
                return Err(MakeBreakError::InvalidTeamCount.into());
            }
            let num_breaking_teams = team_ranking.len() / 3 * 2;
            let teams = team_ranking.into_iter().take((num_breaking_teams) as usize).collect_vec();
            let speakers = find_speakers_not_in_teams(&teams, &speaker_ranking, &speaker_info.team_members);

            break_.breaking_teams = teams;
            break_.breaking_speakers = speakers;
        },
        open_tab_entities::domain::tournament_plan_node::BreakConfig::KnockoutBreak | open_tab_entities::domain::tournament_plan_node::BreakConfig::TeamOnlyKnockoutBreak | open_tab_entities::domain::tournament_plan_node::BreakConfig::BestSpeakerOnlyBreak => {
            let relevant_round = preceding_rounds.first().ok_or(MakeBreakError::KOBreakConditionNotMet)?;

            let mut break_team_ids = vec![];
            let mut best_speaker_ids = vec![];
            let mut team_breaking_ids = vec![];

            let debates = TournamentDebate::get_all_in_rounds(db, vec![*relevant_round]).await?.pop().unwrap();
            let ballots = Ballot::get_many(db, debates.iter().sorted_by_key(|d| d.index).map(|d| d.ballot_id).collect()).await?;

            for ballot in ballots {
                let winning_role = match (ballot.government_total(), ballot.opposition_total()) {
                    (Some(gov_total), Some(opp_total)) => {
                        match gov_total.total_cmp(&opp_total) {
                            Ordering::Equal => {
                                if thread_rng().gen() {
                                    SpeechRole::Government
                                }
                                else {
                                    SpeechRole::Opposition
                                }
                            },
                            Ordering::Greater => SpeechRole::Government,
                            Ordering::Less => SpeechRole::Opposition
                        }
                    },
                    _ => return Err(MakeBreakError::KORoundIncompleteRound.into())
                };

                let remaining_speeches = ballot.speeches.iter().filter(
                    |s| s.role != winning_role
                ).collect_vec();

                let best_speech = remaining_speeches.into_iter().sorted_by_cached_key(|s| ordered_float::NotNan::new(s.speaker_score().unwrap_or(0.0)).unwrap() + thread_rng().gen_range(0.0..0.000001)).rev().next().ok_or(MakeBreakError::KORoundIncompleteRound)?;

                if winning_role == SpeechRole::Government {
                    let gov = ballot.government.team.ok_or(MakeBreakError::KORoundIncompleteRound)?;
                    team_breaking_ids.extend(
                        speaker_info.team_members.get(&gov).map(|m| m.clone().into_iter()).into_iter().flatten()
                    );
                    break_team_ids.push(gov);
                }
                else {
                    let opp = ballot.opposition.team.ok_or(MakeBreakError::KORoundIncompleteRound)?;
                    team_breaking_ids.extend(
                        speaker_info.team_members.get(&opp).map(|m| m.clone().into_iter()).into_iter().flatten()
                    );
                    break_team_ids.push(opp);
                }
                best_speaker_ids.push(best_speech.speaker.ok_or(MakeBreakError::KORoundIncompleteRound)?);
            }

            let tab_breaking_speakers = tab.speaker_tab.iter()
            .sorted_by_cached_key(|e| -ordered_float::NotNan::new(e.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap())
            .filter(
                |e| {
                    !best_speaker_ids.contains(&e.speaker_uuid)
                    && !team_breaking_ids.contains(&e.speaker_uuid)
                }
            ).take(debates.len() / 2).collect_vec();

            if tab_breaking_speakers.len() < debates.len() / 2 {
                return Err(MakeBreakError::NotEnoughTeams.into())
            }

            if *config != open_tab_entities::domain::tournament_plan_node::BreakConfig::BestSpeakerOnlyBreak {
                break_.breaking_teams = break_team_ids;
            }
            if *config != open_tab_entities::domain::tournament_plan_node::BreakConfig::TeamOnlyKnockoutBreak {
                break_.breaking_speakers = tab_breaking_speakers.iter().map(|e| e.speaker_uuid).collect();
            }
            if *config == open_tab_entities::domain::tournament_plan_node::BreakConfig::BestSpeakerOnlyBreak {
                break_.breaking_speakers = best_speaker_ids.first().cloned().into_iter().collect();
            }
        },
        open_tab_entities::domain::tournament_plan_node::BreakConfig::TimBreak => {
            if team_ranking.len() < 3 || team_ranking.len() % 3 != 0 {
                return Err(MakeBreakError::InvalidTeamCount.into());
            }
            let num_breaking_teams = team_ranking.len() / 3;
            let breaking_teams = team_ranking.iter().take((num_breaking_teams) as usize).cloned().collect_vec();

            let relevant_round = preceding_rounds.first().ok_or(MakeBreakError::KOBreakConditionNotMet)?;

            let debates = TournamentDebate::get_all_in_rounds(db, vec![*relevant_round]).await?.pop().unwrap();
            let ballots = Ballot::get_many(db, debates.iter().sorted_by_key(|d| d.index).map(|d| d.ballot_id).collect()).await?;

            let mut non_breaking_teams = vec![];

            for ballot in ballots {
                if let Some(gov) = ballot.government.team {
                    if !breaking_teams.contains(&gov) {
                        non_breaking_teams.push(gov);
                    }
                }
                if let Some(opp) = ballot.opposition.team {
                    if !breaking_teams.contains(&opp) {
                        non_breaking_teams.push(opp);
                    }
                }
            }

            let mut breaking_teams : Vec<Uuid> = speaker_info.teams_by_id.iter().filter_map(
                |(team_id, _team)| {
                    if !non_breaking_teams.contains(team_id) {
                        Some(*team_id)
                    }
                    else {
                        None
                    }
                }
            ).collect_vec();

            let speakers = find_speakers_not_in_teams(&breaking_teams, &speaker_ranking, &speaker_info.team_members);
            let team_positions = team_ranking.iter().enumerate().map(|(i, t)| (*t, i)).collect::<HashMap<_, _>>();
            breaking_teams.sort_by_key(|t| team_positions.get(t).unwrap_or(&0));
            break_.breaking_teams = breaking_teams;
            break_.breaking_speakers = speakers;  
        },
        open_tab_entities::domain::tournament_plan_node::BreakConfig::Manual => {
            return Err(MakeBreakError::IsManualBreak.into());
        }
    }

    if let Some(break_id) = break_id {
        break_.uuid = break_id;
    }

    let break_id = break_.uuid;
    break_.break_award_title = suggested_award_title.clone();
    break_.break_award_prestige = suggested_break_award_prestige.clone();

    groups.add(Entity::TournamentBreak(break_));
    let mut original_node = all_nodes.get(&node_id).expect("Guaranteed by db constraints").clone();
    match &mut original_node.config {
        PlanNodeType::Round { .. } => {
            original_node.config = PlanNodeType::Break {
                config: BreakConfig::TabBreak { num_teams: 0, num_non_aligned: 0 },
                break_id: Some(break_id),
                eligible_categories: vec![],
                suggested_award_title: None,
                max_breaking_adjudicator_count: None,
                is_only_award: false,
                suggested_break_award_prestige: None,
                suggested_award_series_key: None
            };
        },
        PlanNodeType::Break { break_id: break_id_ref, .. } => {
            *break_id_ref = Some(break_id);
        },
    };
    groups.add(Entity::TournamentPlanNode(original_node));

    Ok(groups)
}


#[async_trait]
impl ActionTrait for ExecutePlanNodeAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {

        let node = TournamentPlanNode::get(db, self.plan_node).await?;

        let changes = match &node.config {
            open_tab_entities::domain::tournament_plan_node::PlanNodeType::Round { config, rounds } => {
                generate_round_draw(db, self.tournament_id, node.uuid, config, rounds).await?
            },
            open_tab_entities::domain::tournament_plan_node::PlanNodeType::Break { config, break_id, eligible_categories, suggested_award_title, suggested_break_award_prestige, .. } => {
                let participants = Participant::get_all_in_tournament(db, self.tournament_id).await?.into_iter().map(
                    |p| (p.uuid, p)
                ).collect::<HashMap<_, _>>();
                generate_break(db, self.tournament_id, node.uuid, config, break_id.clone(), eligible_categories, suggested_award_title, suggested_break_award_prestige, &participants).await?
            },
        };

        Ok(changes)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use open_tab_entities::{domain::tournament_plan_node::{AdjudicatorEligibilityMode, EligibilityConfig, NonAlignedEligibilityMode, TeamEligibilityMode, TournamentEligibleBreakCategory}, prelude::Participant};
    use sea_orm::prelude::Uuid;

    use super::get_eligiblity_info;

    #[test]
    fn test_break_with_no_eligibility_includes_all_teams() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 0), (101, 0), (102, 0)]),
                (2, vec![(103, 1), (104, 2), (105, 1)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &vec![],
            &participants
        );

        assert_eq!(info.eligible_teams.len(), 3);
        assert_eq!(info.eligible_speakers.len(), 9);
        assert_eq!(info.eligible_adjudicators.len(), 0);
    }

    #[test]
    fn test_all_eligible_only_includes_fully_eligible_teams() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_teams.len(), 1);
        assert_eq!(info.eligible_teams.iter().next().unwrap(), &Uuid::from_u128(1));
    }

    #[test]
    fn test_majority_eligible_includes_full_and_majority_teams() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 1), (104, 2), (105, 1)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::MajorityEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_teams.len(), 2);
        assert_eq!(info.eligible_teams.contains(&Uuid::from_u128(1)), true);
        assert_eq!(info.eligible_teams.contains(&Uuid::from_u128(2)), true);
    }

    #[test]
    fn test_any_eligible_includes_any_teams() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AnyEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_teams.len(), 2);
    }

    #[test]
    fn test_all_non_aligned_includes_all_non_aligned() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_speakers.len(), 9);
    }

    #[test]
    fn test_all_eligible_non_aligned() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::AllEligible,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_speakers.len(), 4);
    }

    #[test]
    fn test_all_eligible_in_eligible_team() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AnyEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::AllEligibleInEligibleTeams,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_speakers.len(), 4);
    }

    #[test]
    fn test_all_speakers_eligibile() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_speakers.len(), 9);
    }

    #[test]
    fn test_all_in_eligible_team() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 0)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 0)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AnyEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::AllInEligibleTeams,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_speakers.len(), 6);
    }

    #[test]
    fn test_multiple_categories_make_union() {
        let participants = mock_teams(
            vec![
                (1, vec![(100, 1), (101, 1), (102, 1)]),
                (2, vec![(103, 0), (104, 0), (105, 0)]),
                (3, vec![(106, 1), (107, 2), (108, 3)])
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
                (2, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AnyEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_teams.len(), 2);
    }

    #[test]
    fn test_break_all_adjudicators() {
        let participants = mock_adjudicators(
            vec![
                (1, 1),
                (2, 1),
                (3, 2),
                (4, 3)
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_adjudicators.len(), 4);
    }

    #[test]
    fn test_break_only_category_adjudicators() {
        let participants = mock_adjudicators(
            vec![
                (1, 1),
                (2, 1),
                (3, 2),
                (4, 3)
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::AllEligible,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_adjudicators.len(), 2);
    }

    #[test]
    fn test_break_only_category_adjudicators_with_do_not_restrict() {
        let participants = mock_adjudicators(
            vec![
                (1, 1),
                (2, 1),
                (3, 2),
                (4, 3)
            ]
        );

        let info = get_eligiblity_info(
            &mock_categories(vec![
                (1, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::AllEligible,
                }),
                (2, EligibilityConfig {
                    team_eligibility_mode: TeamEligibilityMode::AllEligible,
                    non_aligned_eligibility_mode: NonAlignedEligibilityMode::DoNotRestrict,
                    adjudicator_eligibility_mode: AdjudicatorEligibilityMode::DoNotRestrict,
                }),
            ]),
            &participants
        );

        assert_eq!(info.eligible_adjudicators.len(), 2);
    }


    fn mock_categories(configs: Vec<(u128, EligibilityConfig)>) -> Vec<TournamentEligibleBreakCategory> {
        configs.into_iter().map(
            |(category_id, config)| {
                TournamentEligibleBreakCategory {
                    category_id: Uuid::from_u128(category_id),
                    config,
                }
            }
        ).collect()
    }

    fn mock_teams(teams: Vec<(u128, Vec<(u128, u128)>)>) -> HashMap<Uuid, Participant> {
        let participants = teams.into_iter().flat_map(
            |(team_id, members)| {
                members.into_iter().map(
                    move |(member_id, category_id)| {
                        mock_speaker_with_category(
                            Uuid::from_u128(member_id),
                            Uuid::from_u128(category_id),
                            Uuid::from_u128(team_id)
                        )
                    }
                )
            }
        );

        participants.map(
            |p| (p.uuid, p)
        ).collect()
    }

    fn mock_adjudicators(adjudicators: Vec<(u128, u128)>) -> HashMap<Uuid, Participant> {
        let participants = adjudicators.into_iter().map(
            |(adjudicator_id, category_id)| {
                mock_adjudicator_with_category(
                    Uuid::from_u128(adjudicator_id),
                    Uuid::from_u128(category_id)
                )
            }
        );

        participants.map(
            |p| (p.uuid, p)
        ).collect()
    }

    fn mock_adjudicator_with_category(uuid: Uuid, category: Uuid) -> Participant {
        let mut p = Participant::new_with_uuid(
            uuid,
            "Test".into(),
            open_tab_entities::prelude::ParticipantRole::Adjudicator(
                open_tab_entities::prelude::Adjudicator {
                    ..Default::default()
                }
            ),
            Uuid::from_u128(1)
        );
        if category != Uuid::nil() {
            p.break_category_id = Some(category);
        }
        p
    }

    fn mock_speaker_with_category(uuid: Uuid, category: Uuid, team_id: Uuid) -> Participant {
        let mut p = Participant::new_with_uuid(
            uuid,
            "Test".into(),
            open_tab_entities::prelude::ParticipantRole::Speaker(
                open_tab_entities::prelude::Speaker {
                    team_id: Some(team_id),
                }
            ),
            Uuid::from_u128(1)
        );
        if category != Uuid::nil() {
            p.break_category_id = Some(category);
        }
        p
    }
}