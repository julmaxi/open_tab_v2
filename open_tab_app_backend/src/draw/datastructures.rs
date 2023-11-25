use std::collections::HashMap;

use itertools::Itertools;
use open_tab_entities::{prelude::{Ballot, BallotTeam, Speech, SpeechRole}, domain::{self, entity::LoadEntity}};
use sea_orm::{prelude::Uuid};


use crate::draw_view::DrawBallot;




#[derive(Debug, Clone)]
pub struct AdjudicatorInfo {
    pub id: Uuid,
    pub feedback_skill: i32,
    pub moderation_skill: i32,

    pub discussion_skill: i32,

    pub bias: f32,
    pub variance: f32,
}


#[derive(Debug, Clone)]
pub struct DebateInfo {
    pub id: Uuid,
    pub government: Option<Uuid>,
    pub opposition: Option<Uuid>,
    pub chair: Option<Uuid>,
    pub wings: Vec<Uuid>,
    pub president: Option<Uuid>,
    pub non_aligned_speakers: Vec<Uuid>,
}

impl From<Ballot> for DebateInfo {
    fn from(ballot: Ballot) -> Self {
        Self {
            id: ballot.uuid,
            government: ballot.government.team,
            opposition: ballot.opposition.team,
            chair: ballot.adjudicators.get(0).cloned(),
            wings: ballot.adjudicators.iter().skip(1).cloned().collect_vec(),
            non_aligned_speakers: ballot.speeches.iter().filter_map(|s| match s.role {
                domain::ballot::SpeechRole::NonAligned => Some(s.speaker).flatten(),
                _ => None
            }).collect_vec(),
            president: ballot.president
        }
    }
}

impl Into<Ballot> for DebateInfo {
    fn into(self) -> Ballot {
        let mut speeches = vec![
            (open_tab_entities::domain::ballot::SpeechRole::Government),
            (open_tab_entities::domain::ballot::SpeechRole::Opposition),
        ].into_iter().flat_map(
            |role| {
                (0..3).map(
                    move |position| Speech {
                        speaker: None,
                        role,
                        position,
                        scores: HashMap::new()
                    }
                )
            }
        ).collect_vec();
        speeches.extend(
            self.non_aligned_speakers.into_iter().enumerate().map(
                |(idx, u)| Speech {
                    speaker: Some(u),
                    role: SpeechRole::NonAligned,
                    position: idx as u8,
                    scores: HashMap::new()
                }
            )
        );
        Ballot {
            uuid: self.id,
            speeches: speeches,
            government: BallotTeam {
                team: self.government,
                ..Default::default()
            },
            opposition: BallotTeam {
                team: self.opposition,
                ..Default::default()
            },
            adjudicators: self.chair.iter().chain(
                self.wings.iter()
            ).cloned().collect_vec(),
            president: self.president
        }
    }
}

impl From<&DrawBallot> for DebateInfo {
    fn from(ballot: &DrawBallot) -> Self {
        Self {
            id: ballot.uuid,
            government: ballot.government.as_ref().map(|g| g.uuid),
            opposition: ballot.opposition.as_ref().map(|g| g.uuid),
            chair: ballot.adjudicators.get(0).map(|a| a.adjudicator.uuid),
            wings: ballot.adjudicators.iter().skip(1).map(|a| a.adjudicator.uuid).collect_vec(),
            non_aligned_speakers: ballot.non_aligned_speakers.iter().map(|s| s.uuid).collect_vec(),
            president: ballot.president.as_ref().map(|p| p.adjudicator.uuid)
        }
    }
}

impl From<DrawBallot> for DebateInfo {
    fn from(ballot: DrawBallot) -> Self {
        Self::from(&ballot)
    }
}

#[derive(Debug, Clone)]
pub struct RoundInfo {
    pub id: Uuid,
    pub debates: Vec<DebateInfo>,
    pub is_silent: bool
}

impl RoundInfo {
    pub async fn load_from_rounds<C>(db: &C, round_ids: Vec<Uuid>) -> Result<Vec<Self>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let rounds = domain::round::TournamentRound::get_many(db, round_ids.clone()).await?.into_iter().sorted_by_key(|r| r.index).collect_vec();
        let ballots = Ballot::get_all_in_rounds(db, round_ids).await?;

        let rounds = rounds.into_iter().zip(ballots.into_iter()).map(
            |(round_, (_, ballots))| {
                let debates = ballots.into_iter().map(DebateInfo::from).collect_vec();

                RoundInfo {
                    id: round_.uuid,
                    debates,
                    is_silent: round_.is_silent
                }
            }
        ).collect_vec();        

        Ok(rounds)
    }
}