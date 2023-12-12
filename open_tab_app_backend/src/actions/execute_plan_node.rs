use std::{sync::Arc, collections::{HashSet, HashMap}, cmp::Ordering};

use itertools::{Itertools, izip, repeat_n};
use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{tournament_break::TournamentBreak, tournament_venue::TournamentVenue, tournament_plan_node::{TournamentPlanNode, RoundGroupConfig, PlanNodeType, BreakConfig}, entity::LoadEntity, tournament_plan_edge::TournamentPlanEdge}, EntityType, tab::TeamRoundRole, derived_models::{BreakNodeBackgroundInfo, NodeExecutionError}};

use rand::{thread_rng, Rng};
use sea_orm::prelude::*;

use crate::{draw::{PreliminaryRoundGenerator, PreliminariesDrawMode, evaluation::DrawEvaluator, preliminary::{RoundGenerationContext, DrawTeamInfo}, tab_draw::{pair_teams, pair_speakers, TeamPair, assign_teams}, flow_optimization::{OptimizationState, OptimizationOptions}}, TournamentParticipantsInfo, draw_view::{DrawBallot, DrawTeam, DrawSpeaker, DrawAdjudicator, SetDrawAdjudicator}, views};
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
    let mut changes = EntityGroup::new();

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
            PlanNodeType::Break { config: _, break_id } => {
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
            }
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

    // We maintain a list of all inserted rounds, so we can reindex them later. This is
    // why this is mutable.
    let other_rounds = TournamentRound::get_all_in_tournament(db, tournament_id).await?.into_iter().filter(
        |r| !existing_rounds.contains(&r.uuid)
    ).collect_vec();
    let draw_evaluator = DrawEvaluator::new_from_rounds(db, tournament_id, &other_rounds).await?;

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
            changes.delete(EntityType::TournamentRound, round.uuid);
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

            let ballots = generator.generate_draw_for_rounds(&context, rounds.iter().collect(), &draw_evaluator)?;
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

    let mut optimization_state = OptimizationState::load_from_rounds_and_draw_ballots(db, tournament_id, rounds.iter().zip(ballots.iter()).collect(), Arc::new(OptimizationOptions::default()), Arc::new(draw_evaluator)).await?;

    let adjudicators_to_include = if all_adjudicator_ids.len() > 0 {
        Some(&all_adjudicator_ids)
    }
    else {
        None
    };
    optimization_state.update_state_by_assigning_adjudicators(adjudicators_to_include);

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
        else if round_existing_debates.len() > round_new_ballots.len() {
            for debate in round_existing_debates.iter().skip(round_new_ballots.len()) {
                changes.delete(
                    EntityType::TournamentDebate,
                    debate.uuid
                );
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


async fn generate_break<C>(db: &C, tournament_id: Uuid, node_id: Uuid, config: &BreakConfig, break_id: Option<Uuid>) -> Result<EntityGroup, anyhow::Error> where C: sea_orm::ConnectionTrait {
    let mut groups = EntityGroup::new();

    let break_background = BreakNodeBackgroundInfo::load_for_break_node(db, tournament_id, node_id).await?;

    let all_nodes = break_background.all_nodes;
    let preceding_rounds = break_background.preceding_rounds;
    let _relevant_break = break_background.relevant_break_id;

    let speaker_info = TournamentParticipantsInfo::load(db, tournament_id).await?;

    let tab = views::tab_view::TabView::load_from_rounds(
        db,
        preceding_rounds.clone(),
        &speaker_info
    ).await?;

    let team_ranking = tab.team_tab.iter().sorted_by_key(
        |t: &&crate::tab_view::TeamTabEntry| ordered_float::NotNan::new(t.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap()
    ).rev().map(|t| t.team_uuid).collect_vec();

    let speaker_ranking = tab.speaker_tab.iter().sorted_by_key(
        |s: &&crate::tab_view::SpeakerTabEntry| ordered_float::NotNan::new(s.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap()
    ).rev().map(|s| s.speaker_uuid).collect_vec();

    let mut break_ = TournamentBreak::new(tournament_id);

    match config {
        open_tab_entities::domain::tournament_plan_node::BreakConfig::TabBreak { num_debates } => {
            let teams = team_ranking.into_iter().take((num_debates * 2) as usize).collect_vec();
            if teams.len() < (num_debates * 2) as usize {
                return Err(MakeBreakError::NotEnoughTeams.into());
            }
            let speakers = find_speakers_not_in_teams(&teams, &speaker_ranking, &speaker_info.team_members);
            let speakers = speakers.into_iter().take((num_debates * 3) as usize).collect_vec();

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
        open_tab_entities::domain::tournament_plan_node::BreakConfig::KnockoutBreak => {
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
            .sorted_by_key(|e| -ordered_float::NotNan::new(e.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap())
            .filter(
                |e| {
                    !best_speaker_ids.contains(&e.speaker_uuid)
                    && !team_breaking_ids.contains(&e.speaker_uuid)
                }
            ).take(debates.len() / 2).collect_vec();

            if tab_breaking_speakers.len() < debates.len() / 2 {
                return Err(MakeBreakError::NotEnoughTeams.into())
            }

            break_.breaking_teams = break_team_ids;
            break_.breaking_speakers = tab_breaking_speakers.iter().map(|e| e.speaker_uuid).chain(best_speaker_ids.into_iter()).collect();
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

    groups.add(Entity::TournamentBreak(break_));
    let mut original_node = all_nodes.get(&node_id).expect("Guaranteed by db constraints").clone();
    original_node.config = PlanNodeType::Break { config: config.clone(), break_id: Some(break_id) };
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
            open_tab_entities::domain::tournament_plan_node::PlanNodeType::Break { config, break_id } => {
                generate_break(db, self.tournament_id, node.uuid, config, break_id.clone()).await?
            },
        };

        Ok(changes)
    }
}