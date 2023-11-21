use std::{error::Error, collections::HashMap, cmp::Ordering};

use itertools::{Itertools};
use async_trait::async_trait;
use open_tab_entities::{prelude::*, domain::{tournament_break::TournamentBreakSourceRoundType, entity::LoadEntity}};

use rand::{thread_rng, Rng};
use sea_orm::prelude::*;

use crate::{views, TournamentParticipantsInfo};
use serde::{Serialize, Deserialize};
use open_tab_entities::domain::tournament_break::TournamentBreak;
use super::ActionTrait;

use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakeBreakAction {
    break_id: Uuid
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
}


struct FoldingPairIterator {
    num_items: u64,
    curr_item: u64,
}

impl FoldingPairIterator {
    fn new(num_items: u64) -> Self {
        Self {
            num_items,
            curr_item: 0
        }
    }
}

impl Iterator for FoldingPairIterator {
    type Item = (u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_item >= self.num_items {
            return None;
        }

        let first = self.curr_item;
        let second = self.num_items - self.curr_item - 1;

        self.curr_item += 1;

        Some((first, second))
    }
}


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


#[async_trait]
impl ActionTrait for MakeBreakAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: ConnectionTrait {
        let mut groups = EntityGroup::new();

        let mut break_ = TournamentBreak::get_many(db, vec![self.break_id]).await?.pop().unwrap();

        let speaker_info = TournamentParticipantsInfo::load(db, break_.tournament_id).await?;

        let tab = views::tab_view::TabView::load_from_rounds(
            db,
            break_.source_rounds.iter().map(|r| r.uuid).collect(),
            &speaker_info
        ).await?;

        let team_ranking = tab.team_tab.iter().sorted_by_key(
            |t: &&crate::tab_view::TeamTabEntry| ordered_float::NotNan::new(t.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap()
        ).rev().map(|t| t.team_uuid).collect_vec();

        let speaker_ranking = tab.speaker_tab.iter().sorted_by_key(
            |s: &&crate::tab_view::SpeakerTabEntry| ordered_float::NotNan::new(s.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap()
        ).rev().map(|s| s.speaker_uuid).collect_vec();

        match break_.break_type {
            open_tab_entities::domain::tournament_break::BreakType::TabBreak { num_debates } => {
                let teams = team_ranking.into_iter().take((num_debates * 2) as usize).collect_vec();
                if teams.len() < (num_debates * 2) as usize {
                    return Err(MakeBreakError::NotEnoughTeams.into());
                }
                let speakers = find_speakers_not_in_teams(&teams, &speaker_ranking, &speaker_info.team_members);

                break_.breaking_teams = teams;
                break_.breaking_speakers = speakers.into_iter().take(num_debates as usize * 3).collect();

                groups.add(Entity::TournamentBreak(break_));
            },
            open_tab_entities::domain::tournament_break::BreakType::TwoThirdsBreak => {
                if team_ranking.len() < 3 || team_ranking.len() % 3 != 0 {
                    return Err(MakeBreakError::InvalidTeamCount.into());
                }
                let num_breaking_teams = team_ranking.len() / 3 * 2;
                let teams = team_ranking.into_iter().take((num_breaking_teams) as usize).collect_vec();
                let speakers = find_speakers_not_in_teams(&teams, &speaker_ranking, &speaker_info.team_members);

                break_.breaking_teams = teams;
                break_.breaking_speakers = speakers;

                groups.add(Entity::TournamentBreak(break_));
            },
            open_tab_entities::domain::tournament_break::BreakType::KOBreak => {
                let relevant_round = break_.source_rounds.iter().find(|r| r.break_type == TournamentBreakSourceRoundType::Knockout).ok_or(MakeBreakError::KOBreakConditionNotMet)?;

                let mut break_team_ids = vec![];
                let mut best_speaker_ids = vec![];
                let mut team_breaking_ids = vec![];

                let debates = TournamentDebate::get_all_in_rounds(db, vec![relevant_round.uuid]).await?.pop().unwrap();
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
                .sorted_by_key(|e| ordered_float::NotNan::new(e.total_score + thread_rng().gen_range(0.0..0.000001)).unwrap())
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
                break_.breaking_speakers = tab_breaking_speakers.iter().map(|e| e.speaker_uuid).collect();

                groups.add(Entity::TournamentBreak(break_));
            },
            open_tab_entities::domain::tournament_break::BreakType::TimBreak => todo!(),
        }

        Ok(groups)
    }
}