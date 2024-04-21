use std::{any, collections::HashMap, thread::scope};

use itertools::Itertools;
use sea_orm::{schema, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{self, ballot::Ballot, entity::LoadEntity, feedback_form::{FeedbackForm, FeedbackFormVisibility, FeedbackSourceRole, FeedbackTargetRole}, round::TournamentRound};

#[derive(Debug, Clone)]
pub struct FeedbackRequest {
    pub target_id: Uuid,
    pub source_id: SourceId,
    pub source_role: FeedbackSourceRole,
    pub target_role: FeedbackTargetRole,
}



#[derive(Debug, Clone)]
pub struct DebateFeedbackRequest {
    pub target_id: Uuid,
    pub source_id: SourceId,
    pub source_role: FeedbackSourceRole,
    pub target_role: FeedbackTargetRole,
    pub debate_id: Uuid,
}


#[derive(Debug, Clone)]
pub struct DebateFeedbackSubmissionInfo {
    pub target_id: Uuid,
    pub source_id: SourceId,
    pub source_role: FeedbackSourceRole,
    pub target_role: FeedbackTargetRole,
    pub debate_id: Uuid,
    pub submissions: Vec<Uuid>
}


#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Hash)]
#[serde(tag="type")]
pub enum SourceId {
    Participant{uuid: Uuid},
    Team{uuid: Uuid}
}

pub struct FeedbackProgressMatrix {
    pub submission_info_by_participant: HashMap<Uuid, Vec<DebateFeedbackSubmissionInfo>>,
    pub submission_info_by_team: HashMap<Uuid, Vec<DebateFeedbackSubmissionInfo>>,
}

impl FeedbackProgressMatrix {
    pub async fn from_tournament<C>(db: &C, tournament_id: Uuid) -> anyhow::Result<FeedbackProgressMatrix> where C: ConnectionTrait {
        let rounds = domain::round::TournamentRound::get_all_in_tournament(db, tournament_id).await?;
        let feedback_directions = get_tournament_feedback_directions(db, tournament_id).await?;

        let debates = domain::debate::TournamentDebate::get_all_in_rounds(db, rounds.iter().map(|u| u.uuid).collect_vec()).await?;

        let debates = debates.into_iter().flatten().collect_vec();

        let ballots = Ballot::get_many(db, debates.iter().map(|u| u.ballot_id).collect_vec()).await?;
        
        let all_requests = ballots.iter().zip(debates.iter().map(|d| d.uuid)).flat_map(|(b, debate_id)| {
            let requests = get_feedback_requests_from_ballot(b, &feedback_directions);
            requests.into_iter().map(move |r| {
                DebateFeedbackRequest {
                    target_id: r.target_id,
                    source_id: r.source_id,
                    source_role: r.source_role,
                    target_role: r.target_role,
                    debate_id: debate_id
                }
            })
        }).collect_vec();

        dbg!(&all_requests);

        let submissions = feedback_requests_to_submissions(db, all_requests).await?;
        
        let mut submissions_by_participant = HashMap::new();
        let mut submissions_by_team = HashMap::new();

        for submission in submissions {
            match submission.source_id {
                SourceId::Participant{uuid} => {
                    submissions_by_participant.entry(uuid).or_insert_with(|| vec![]).push(submission);
                },
                SourceId::Team{uuid} => {
                    submissions_by_team.entry(uuid).or_insert_with(|| vec![]).push(submission);
                }
            }
        }
        Ok(FeedbackProgressMatrix {
            submission_info_by_participant: submissions_by_participant,
            submission_info_by_team: submissions_by_team
        })
    }
}

pub async fn get_tournament_feedback_directions<C>(db: &C, tournament_id: Uuid) -> anyhow::Result<Vec<(FeedbackSourceRole, FeedbackTargetRole)>> where C: ConnectionTrait {
    let all_feedback_forms = FeedbackForm::get_all_in_tournament(db, tournament_id).await?;

    let overall_visibility = all_feedback_forms.iter().fold(
        Default::default(),
        |acc : FeedbackFormVisibility, val| {
            acc | &val.visibility
        }
    );

    Ok(overall_visibility.to_feedback_direction_pairs())
}

pub fn get_feedback_requests_from_ballot(ballot: &Ballot, directions: &Vec<(FeedbackSourceRole, FeedbackTargetRole)>) -> Vec<FeedbackRequest> {
    let mut out = vec![];

    for direction in directions {
        match direction {
            (FeedbackSourceRole::Chair, FeedbackTargetRole::Wing) => {
                if ballot.adjudicators.len() > 1 {
                    for adj in &ballot.adjudicators[1..] {
                        out.push(FeedbackRequest {
                            target_id: *adj,
                            source_id: SourceId::Participant{uuid: ballot.adjudicators[0]},
                            source_role: FeedbackSourceRole::Chair,
                            target_role: FeedbackTargetRole::Wing
                        });
                    }
                }
            },
            (FeedbackSourceRole::Chair, FeedbackTargetRole::President) => {
                if ballot.adjudicators.len() > 0 {
                    if let Some(president) = ballot.president {
                        out.push(FeedbackRequest {
                            target_id: president,
                            source_id: SourceId::Participant{uuid: ballot.adjudicators[0]},
                            source_role: FeedbackSourceRole::Chair,
                            target_role: FeedbackTargetRole::President
                        });        
                    }
                }
            },
            (FeedbackSourceRole::Wing, FeedbackTargetRole::Chair) => {
                if ballot.adjudicators.len() > 1 {
                    for adj in &ballot.adjudicators[1..] {
                        out.push(FeedbackRequest {
                            target_id: ballot.adjudicators[0],
                            source_id: SourceId::Participant{uuid: *adj},
                            source_role: FeedbackSourceRole::Wing,
                            target_role: FeedbackTargetRole::Chair
                        });
                    }
                }
            },
            (FeedbackSourceRole::Wing, FeedbackTargetRole::Wing) => {
                if ballot.adjudicators.len() > 2 {
                    let wings = &ballot.adjudicators[1..];
                    for pair in wings.iter().combinations(2) {
                        out.push(FeedbackRequest {
                            target_id: *pair[0],
                            source_id: SourceId::Participant{uuid: *pair[1]},
                            source_role: FeedbackSourceRole::Wing,
                            target_role: FeedbackTargetRole::Wing
                        });
                        out.push(FeedbackRequest {
                            target_id: *pair[1],
                            source_id: SourceId::Participant{uuid: *pair[0]},
                            source_role: FeedbackSourceRole::Wing,
                            target_role: FeedbackTargetRole::Wing
                        });
                    }
                }
            },
            (FeedbackSourceRole::Wing, FeedbackTargetRole::President) => {
                if ballot.adjudicators.len() > 1 {
                    if let Some(president) = ballot.president {
                        for adj in &ballot.adjudicators[1..] {
                            out.push(FeedbackRequest {
                                target_id: president,
                                source_id: SourceId::Participant{uuid: *adj},
                                source_role: FeedbackSourceRole::Wing,
                                target_role: FeedbackTargetRole::President
                            });
                        }
                    }
                }
            },
            (FeedbackSourceRole::President, FeedbackTargetRole::Chair) => {
                if let Some(president) = ballot.president {
                    if ballot.adjudicators.len() > 0 {
                        out.push(FeedbackRequest {
                            target_id: ballot.adjudicators[0],
                            source_id: SourceId::Participant{uuid: president},
                            source_role: FeedbackSourceRole::President,
                            target_role: FeedbackTargetRole::Chair
                        });
                    }
                }
            },
            (FeedbackSourceRole::President, FeedbackTargetRole::Wing) => {
                if let Some(president) = ballot.president {
                    if ballot.adjudicators.len() > 1 {
                        for adj in &ballot.adjudicators[1..] {
                            out.push(FeedbackRequest {
                                target_id: *adj,
                                source_id: SourceId::Participant{uuid: president},
                                source_role: FeedbackSourceRole::President,
                                target_role: FeedbackTargetRole::Wing
                            });
                        }
                    }
                }
            },
            (FeedbackSourceRole::Team, FeedbackTargetRole::Chair) => {
                if let Some(team) = ballot.government.team {
                    if ballot.adjudicators.len() > 0 {
                        out.push(FeedbackRequest {
                            target_id: ballot.adjudicators[0],
                            source_id: SourceId::Team{uuid: team},
                            source_role: FeedbackSourceRole::Team,
                            target_role: FeedbackTargetRole::Chair
                        });
                    }
                }
                if let Some(team) = ballot.opposition.team {
                    if ballot.adjudicators.len() > 0 {
                        out.push(FeedbackRequest {
                            target_id: ballot.adjudicators[0],
                            source_id: SourceId::Team{uuid: team},
                            source_role: FeedbackSourceRole::Team,
                            target_role: FeedbackTargetRole::Chair
                        });
                    }
                }
            },
            (FeedbackSourceRole::Team, FeedbackTargetRole::Wing) => {
                if let Some(team) = ballot.government.team {
                    if ballot.adjudicators.len() > 1 {
                        for adj in &ballot.adjudicators[1..] {
                            out.push(FeedbackRequest {
                                target_id: *adj,
                                source_id: SourceId::Team{uuid: team},
                                source_role: FeedbackSourceRole::Team,
                                target_role: FeedbackTargetRole::Wing
                            });
                        }
                    }
                }
                if let Some(team) = ballot.opposition.team {
                    if ballot.adjudicators.len() > 1 {
                        for adj in &ballot.adjudicators[1..] {
                            out.push(FeedbackRequest {
                                target_id: *adj,
                                source_id: SourceId::Team{uuid: team},
                                source_role: FeedbackSourceRole::Team,
                                target_role: FeedbackTargetRole::Wing
                            });
                        }
                    }
                }
            },
            (FeedbackSourceRole::Team, FeedbackTargetRole::President) => {
                if let Some(president) = ballot.president {
                    if let Some(team) = ballot.government.team {
                        out.push(FeedbackRequest {
                            target_id: president,
                            source_id: SourceId::Team{uuid: team},
                            source_role: FeedbackSourceRole::Team,
                            target_role: FeedbackTargetRole::President
                        });
                    }
                    if let Some(team) = ballot.opposition.team {
                        out.push(FeedbackRequest {
                            target_id: president,
                            source_id: SourceId::Team{uuid: team},
                            source_role: FeedbackSourceRole::Team,
                            target_role: FeedbackTargetRole::President
                        });
                    }
                }
            },
            (FeedbackSourceRole::NonAligned, FeedbackTargetRole::Chair) => {
                for non_aligned in ballot.speeches.iter().filter(|s| s.role == domain::ballot::SpeechRole::NonAligned).filter_map(|s| s.speaker) {
                    if ballot.adjudicators.len() > 0 {
                        out.push(FeedbackRequest {
                            target_id: ballot.adjudicators[0],
                            source_id: SourceId::Participant{uuid: non_aligned},
                            source_role: FeedbackSourceRole::NonAligned,
                            target_role: FeedbackTargetRole::Chair
                        });
                    }
                }
            },
            (FeedbackSourceRole::NonAligned, FeedbackTargetRole::Wing) => {
                if ballot.adjudicators.len() > 1 {
                    for non_aligned in ballot.speeches.iter().filter(|s| s.role == domain::ballot::SpeechRole::NonAligned).filter_map(|s| s.speaker) {
                        for adj in &ballot.adjudicators[1..] {
                            out.push(FeedbackRequest {
                                target_id: *adj,
                                source_id: SourceId::Participant{uuid: non_aligned},
                                source_role: FeedbackSourceRole::NonAligned,
                                target_role: FeedbackTargetRole::Wing
                            });
                        }
                    }
                }
            },
            (FeedbackSourceRole::NonAligned, FeedbackTargetRole::President) => {
                if let Some(president) = ballot.president {
                    for non_aligned in ballot.speeches.iter().filter(|s| s.role == domain::ballot::SpeechRole::NonAligned).filter_map(|s| s.speaker) {
                        out.push(FeedbackRequest {
                            target_id: president,
                            source_id: SourceId::Participant{uuid: non_aligned},
                            source_role: FeedbackSourceRole::NonAligned,
                            target_role: FeedbackTargetRole::President
                        });
                    }
                }
            },
            _ => {
                panic!("Invalid feedback direction: {:?}", direction);
            }
        }
    }

    out
}

pub async fn feedback_requests_to_submissions<C>(db: &C, requests: Vec<DebateFeedbackRequest>) -> anyhow::Result<Vec<DebateFeedbackSubmissionInfo>> where C: ConnectionTrait {
    let relevant_debates = requests.iter().map(|r| r.debate_id).unique().collect_vec();

    let submissions = crate::schema::feedback_response::Entity::find().filter(crate::schema::feedback_response::Column::SourceDebateId.is_in(relevant_debates)).all(db).await?;

    let submission_map = submissions.into_iter().into_group_map_by(|submission| {
        let source = match (submission.source_participant_id, submission.source_team_id) {
            (Some(participant_id), None) => {
                SourceId::Participant { uuid: participant_id }
            },
            (None, Some(source_team_id)) => {
                SourceId::Team { uuid: source_team_id }
            },
            _ => {
                panic!("No valid source in feedback submission");
            }
        };
        (submission.source_debate_id, source, submission.target_participant_id)
    });

    let out = requests.into_iter().map(|request| {
        DebateFeedbackSubmissionInfo {
            target_id: request.target_id,
            source_id: request.source_id,
            source_role: request.source_role,
            target_role: request.target_role,
            debate_id: request.debate_id,
            submissions: submission_map.get(&(request.debate_id, request.source_id, request.target_id)).map(|subs| subs.iter().map(|s| s.uuid).collect_vec()).unwrap_or_default()
        }
    }).collect();

    Ok(out)
}