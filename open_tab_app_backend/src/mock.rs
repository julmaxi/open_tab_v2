
use std::{collections::HashMap, error::Error, hash::Hash, fmt::{Display, Formatter}};

use migration::{MigratorTrait, async_trait::async_trait};
use open_tab_entities::{EntityGroups, domain::{tournament::Tournament, ballot::SpeechRole}, schema::{adjudicator, self}};
use sea_orm::{prelude::*, Statement, Database};
use open_tab_entities::prelude::*;
use itertools::{Itertools, izip};
use serde::{Serialize, Deserialize};

use crate::{View, draw_view::{DrawDebate, DrawBallot, DrawView}, Action};

pub fn make_mock_tournament() -> EntityGroups {
    return make_mock_tournament_with_options(false);
}

pub fn make_mock_tournament_with_options(deterministic_uuids: bool) -> EntityGroups {
    let mut groups = EntityGroups::new();

    let tournament_uuid = if deterministic_uuids {Uuid::from_u128(1)} else {Uuid::new_v4()};
    groups.add(Entity::Tournament(Tournament {
        uuid: tournament_uuid,
        ..Default::default()
    }));
    
    let teams = (0..27).map(|i| {
        let uuid = if deterministic_uuids {Uuid::from_u128(1000 + i)} else {Uuid::new_v4()};
        let name = format!("Team {}", i);
        open_tab_entities::domain::team::Team {
            uuid,
            name,
            tournament_id: tournament_uuid,
        }
    }).collect_vec();

    let speakers = teams.iter().enumerate().map(|(team_idx, team)| {
        let members = (0..3).map(|i| {
            let uuid = if deterministic_uuids {Uuid::from_u128(2000 + (team_idx as u128) * 10 + i)} else {Uuid::new_v4()};
            let name = format!("Speaker {}", uuid);
            Participant {
                uuid,
                name,
                tournament_id: tournament_uuid,
                role: ParticipantRole::Speaker(Speaker { team_id: Some(team.uuid) }),
            }
        }).collect_vec();

        members
    }).collect_vec();

    let adjudicators = (0..27).map(|i| {
        let uuid = if deterministic_uuids {Uuid::from_u128(3000 + i)} else {Uuid::new_v4()};
        let name = format!("Adjudicator {}", uuid);
            Participant {
                uuid,
                name,
                tournament_id: tournament_uuid,
                role: ParticipantRole::Adjudicator(Adjudicator { }),
            }
    }).collect_vec();

    let rounds = (0..3).map(|i| {
        let uuid = if deterministic_uuids {Uuid::from_u128(100 + i)} else {Uuid::new_v4()};
        TournamentRound {
            uuid,
            tournament_id: tournament_uuid,
            index: i as u64,
        }
    }).collect_vec();

    let ballots = rounds.iter().enumerate().map(
        |(round_idx, _round)| {
            (0..9).map(
                |debate_idx| {
                    let uuid = if deterministic_uuids {Uuid::from_u128(400 + (round_idx as u128) * 10 + debate_idx as u128)} else {Uuid::new_v4()};
                    Ballot {
                        uuid,
                        adjudicators: if round_idx < 2 {(0..3).map(|i| adjudicators[debate_idx * 3 + i].uuid).collect()} else {vec![]},
                        government: BallotTeam {
                            // Round 3 has an empty draw for testing purposes
                            team: if round_idx < 2 {Some(teams[debate_idx * 3].uuid)} else {None},
                            ..Default::default()
                        },
                        opposition: BallotTeam {
                            team: if round_idx < 2 {Some(teams[debate_idx * 3 + 1].uuid)} else {None},
                            ..Default::default()
                        },
                        speeches: if round_idx < 2 {(0..3).map(|speaker_idx| {
                            Speech {
                                speaker:Some(speakers[debate_idx][speaker_idx].uuid),
                                role: open_tab_entities::domain::ballot::SpeechRole::NonAligned,
                                position: speaker_idx as u8,
                                scores: HashMap::new(),
                            }
                            }).collect()} else {vec![]},
                        ..Default::default()
                    }
                }
            ).collect_vec()
    }).collect_vec();

    let debates = ballots.iter().enumerate().map(|(round_idx, round_ballots)| {
        let round_debates = round_ballots.iter().enumerate().map(|(debate_idx, ballot)| {
            let uuid = if deterministic_uuids {Uuid::from_u128(200 + debate_idx as u128 + 10 * round_idx as u128)} else {Uuid::new_v4()};
            TournamentDebate {
                uuid,
                round_id: rounds[round_idx].uuid,
                current_ballot_uuid: ballot.uuid,
                index: debate_idx as u64
            }
        }).collect_vec();
        round_debates
    }).collect_vec();

    teams.into_iter().for_each(|team| groups.add(Entity::Team(team)));
    speakers.into_iter().flatten().for_each(|speaker| groups.add(Entity::Participant(speaker)));
    adjudicators.into_iter().for_each(|adjudicator| groups.add(Entity::Participant(adjudicator)));
    rounds.into_iter().for_each(|round| groups.add(Entity::TournamentRound(round)));
    ballots.into_iter().flatten().for_each(|ballot| groups.add(Entity::Ballot(ballot)));
    debates.into_iter().flatten().for_each(|debate| groups.add(Entity::TournamentDebate(debate)));

    groups
}