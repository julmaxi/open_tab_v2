use std::{collections::{HashMap, HashSet}, vec};

use axum::{extract::{Path, State}, Json, Router, routing::{get, post}};
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{derived_models::get_tournament_feedback_directions, domain::{self, ballot::SpeechRole, entity::LoadEntity, feedback_form::{FeedbackSourceRole, FeedbackTargetRole}}, schema::{self}, EntityGroup};
use sea_orm::{DatabaseConnection, TransactionTrait, prelude::*, QuerySelect, QueryOrder};
use serde::{Serialize, Deserialize};

use crate::{auth::{ExtractAuthenticatedUser, MaybeExtractAuthenticatedUser}, response::{handle_error, APIError}, state::AppState};

use open_tab_entities::domain::round::check_release_date;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type")]
pub struct ParticipantInfoResponse {
    pub name: String,
    pub tournament_name: String,
    pub role: ParticipantRoleInfo,
    pub rounds: Vec<ParticipantRoundInfo>,
    pub feedback_submissions: Vec<FeedbackSubmissionInfo>,
    pub expected_reload: Option<chrono::NaiveDateTime>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum ParticipantRoleInfo {
    None,
    Multiple,
    Adjudicator,
    Speaker {team_name: String, team_id: Uuid}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackSubmissionInfo {
    pub target_name: String,
    pub target_id: Uuid,

    pub round_name: String,
    pub round_id: Uuid,
    
    pub debate_id: Uuid,

    pub source_role: FeedbackSourceRole,
    pub source_id: SourceId,
    pub target_role: FeedbackTargetRole,

    pub submitted_responses: Vec<Uuid>
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
#[serde(tag="type")]
pub enum SourceId {
    Participant{uuid: Uuid},
    Team{uuid: Uuid}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VenueInfo {
    uuid: Uuid,
    name: String
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantDebateInfo {
    uuid: Uuid,
    ballot_id: Uuid,
    is_motion_released_to_non_aligned: bool,
    venue: Option<VenueInfo>,
    debate_index: i32,
    round_id: Uuid
}

impl ParticipantDebateInfo {
    pub fn new_from(debate: open_tab_entities::schema::tournament_debate::Model, venue: Option<open_tab_entities::schema::tournament_venue::Model>) -> Self {
        Self {
            uuid: debate.uuid,
            ballot_id: debate.ballot_id,
            is_motion_released_to_non_aligned: debate.is_motion_released_to_non_aligned,
            venue: venue.map(|v| VenueInfo{uuid: v.uuid, name: v.name}),
            debate_index: debate.index,
            round_id: debate.round_id
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum Motion {
    Hidden,
    Shown{motion: String, info_slide: Option<String>}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag="status")]
pub enum RoundStatus {
    Planned,
    DrawReleased,
    WaitingToStart { debate_start_time: Option<DateTime> },
    InProgress,
    Completed
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantRoundInfo {
    pub uuid: Uuid,
    pub name: String,
    pub index: i32,
    pub participant_role: Option<ParticipantRoundRoleInfo>,
    pub motion: Motion,

    #[serde(flatten)]
    pub status: RoundStatus,
    pub is_silent: bool
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundTeamRole {
    Government,
    Opposition
}

impl Into<SpeechRole> for RoundTeamRole {
    fn into(self) -> SpeechRole {
        match self {
            RoundTeamRole::Government => SpeechRole::Government,
            RoundTeamRole::Opposition => SpeechRole::Opposition
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="role")]
pub enum ParticipantRoundRoleInfo {
    NotDrawn,
    TeamSpeaker{
        debate: ParticipantDebateInfo,
        team_role: RoundTeamRole,
        speaker_score: SpeakerScoreInfo,
        team_score: TeamScoreInfo
    },
    NonAlignedSpeaker{
        debate: ParticipantDebateInfo,
        position: i32,
        speaker_score: SpeakerScoreInfo
    },
    Adjudicator{
        debate: ParticipantDebateInfo,
        position: i32
    },
    President {
        debate: ParticipantDebateInfo,
    },
    Multiple
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="score_status")]
pub enum TeamScoreInfo {
    Hidden,
    Shown{total_score: f32, adjudicator_scores: Vec<i16>}
}

impl TeamScoreInfo {
    pub fn from_ballot_and_team_role(ballot: &open_tab_entities::domain::ballot::Ballot, team_role: RoundTeamRole) -> Self {
        let team_score = match team_role {
            RoundTeamRole::Government => ballot.government.team_score(),
            RoundTeamRole::Opposition => ballot.opposition.team_score()
        }.unwrap_or(0.0) as f32;
        let speaker_score = ballot.speeches.iter().filter_map(
            |speech| {
                if speech.role == team_role.clone().into() {
                    speech.speaker_score()
                }
                else {
                    None
                }
            }
        ).sum::<f64>() as f32;

        let adjudicator_team_scores = ballot.adjudicators.iter().filter_map(|a| {
            match team_role {
                RoundTeamRole::Government => &ballot.government,
                RoundTeamRole::Opposition => &ballot.opposition
            }.scores.get(a).map(|s| s.total())
        }).collect_vec();
        Self::Shown{total_score: team_score + speaker_score, adjudicator_scores: adjudicator_team_scores}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="score_status")]
pub enum SpeakerScoreInfo {
    Hidden,
    DidNotParticipate,
    Shown{total_score: f32, adjudicator_scores: Vec<i16>}
}

impl SpeakerScoreInfo {
    fn from_speech(speech: &open_tab_entities::domain::ballot::Speech) -> Self {
        let total_score = speech.speaker_score().unwrap_or(0.0) as f32;
        let adjudicator_scores = speech.scores.iter().map(|(_, score)| score.total()).collect_vec();
        Self::Shown{ total_score, adjudicator_scores }
    }
}

async fn get_participant_info(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<ParticipantInfoResponse>, APIError> {
    let transaction = db.begin().await.map_err(handle_error)?;

    let participant_query_result = open_tab_entities::schema::participant::Entity::find_by_id(participant_id)
    .find_also_related(open_tab_entities::schema::tournament::Entity)
        .one(&transaction).await.map_err(handle_error)?;

    let is_admin = if let Some(participant_query_result) = &participant_query_result  {
        user.check_is_authorized_for_tournament_administration(&transaction, participant_query_result.1.as_ref().expect("Guaranteed by consistency constraints").uuid).await?
    } else {
        false
    };

    if !(is_admin || user.check_is_authorized_as_participant(&transaction, participant_id).await?) {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view this participant"));
        transaction.rollback().await.map_err(handle_error)?;
        return Err(err);
    }

    if participant_query_result.is_none() {
        transaction.rollback().await.map_err(handle_error)?;
        return Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")));
    }

    let (participant, tournament) = participant_query_result.unwrap();
    let tournament = tournament.unwrap(); // Guaranteed by consistency constraints

    let has_access = user.check_is_authorized_for_tournament_administration(&transaction, tournament.uuid).await?;

    let has_access = match has_access {
        true => true,
        false => {
            open_tab_entities::schema::user_participant::Entity::find()
            .filter(
                open_tab_entities::schema::user_participant::Column::UserId.eq(user.uuid)
            ).one(&transaction).await.map_err(handle_error)?.is_some()
        }
    };

    if !has_access {
        transaction.rollback().await.map_err(handle_error)?;
        return Err(APIError::from((StatusCode::FORBIDDEN, "You do not have access to this participant")));
    }
    let current_time = chrono::Utc::now().naive_utc();

    let role = get_participant_role(participant_id, &transaction).await?;

    let all_rounds = open_tab_entities::schema::tournament_round::Entity::find()
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::tournament_round::Relation::Tournament.def())
    .join(sea_orm::JoinType::LeftJoin, open_tab_entities::schema::participant::Relation::Tournament.def().rev())
    .filter(
        open_tab_entities::schema::participant::Column::Uuid.eq(participant_id)
    )
    .order_by_asc(open_tab_entities::schema::tournament_round::Column::Index)
    .all(&transaction).await.map_err(handle_error)?;

    let participant_adjudicator_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotAdjudicator.def())
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::ballot_adjudicator::Column::AdjudicatorId.eq(participant_id)
    ).all(&transaction).await.map_err(handle_error)?;

    let participant_non_aligned_speaker_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotSpeech.def())
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::ballot_speech::Column::SpeakerId.eq(participant_id).and(
            open_tab_entities::schema::ballot_speech::Column::Role.eq(
                open_tab_entities::domain::ballot::SpeechRole::NonAligned.to_str()
            )
        )
    ).all(&transaction).await.map_err(handle_error)?;

    //FIXME: Unelegant
    let participant_gov_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotTeam.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot_team::Relation::Team.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::team::Relation::Speaker.def())
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::speaker::Column::Uuid.eq(participant_id).and(
            open_tab_entities::schema::ballot_team::Column::Role.eq(
                open_tab_entities::domain::ballot::SpeechRole::Government.to_str()
            )
        )
    ).all(&transaction).await.map_err(handle_error)?;

    let participant_opp_debates = open_tab_entities::schema::tournament_debate::Entity::find()
    .inner_join(open_tab_entities::schema::ballot::Entity)
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot::Relation::BallotTeam.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::ballot_team::Relation::Team.def())
    .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::team::Relation::Speaker.def())
    .find_also_related(open_tab_entities::schema::tournament_venue::Entity)
    .filter(
        open_tab_entities::schema::speaker::Column::Uuid.eq(participant_id).and(
            open_tab_entities::schema::ballot_team::Column::Role.eq(
                open_tab_entities::domain::ballot::SpeechRole::Opposition.to_str()
            )
        )
    ).all(&transaction).await.map_err(handle_error)?;

    let all_ballot_ids = participant_adjudicator_debates.iter().map(|d| d.0.ballot_id)
    .chain(participant_non_aligned_speaker_debates.iter().map(|d| d.0.ballot_id))
    .chain(participant_gov_debates.iter().map(|d| d.0.ballot_id))
    .chain(participant_opp_debates.iter().map(|d| d.0.ballot_id))
    .collect::<Vec<_>>();
    let ballot_map = open_tab_entities::domain::ballot::Ballot::get_many(&transaction, all_ballot_ids).await?.into_iter().map(
        |b| (b.uuid, b)
    ).collect::<HashMap<_, _>>();
    
    let show_results_map : HashMap<Uuid, bool> = all_rounds.iter().map(
        |round| {
            let show_results = !round.is_silent && check_release_date(current_time, round.round_close_time);
            (round.uuid, show_results)
        }
    ).collect();

    let participant_adjudicator_debates = participant_adjudicator_debates.into_iter().map(
        |(d, v)| {
            let ballot = ballot_map.get(&d.ballot_id).unwrap();
            if let Some(position) = ballot.adjudicators.iter().position(|a| *a == participant_id) {
                (d.round_id, ParticipantRoundRoleInfo::Adjudicator{
                    debate: ParticipantDebateInfo::new_from(d, v),
                    position: position as i32
                })
            }
            else if ballot.president == Some(participant_id) {
                (d.round_id, ParticipantRoundRoleInfo::President { debate: ParticipantDebateInfo::new_from(d, v) })
            }
            else {
                panic!("Adjudicator {} not found in ballot", participant_id);
            }
         }
    );
    let participant_non_aligned_speaker_debates = participant_non_aligned_speaker_debates.into_iter().map(
        |(d, v)| {
            let ballot = ballot_map.get(&d.ballot_id).unwrap();
            let speech = ballot.speeches.iter().find(|s| s.speaker == Some(participant_id)).unwrap();
            let show_results = show_results_map.get(&d.round_id).unwrap();
            (d.round_id, ParticipantRoundRoleInfo::NonAlignedSpeaker {
                debate: ParticipantDebateInfo::new_from(d, v),
                position: speech.position as i32,
                speaker_score: if *show_results {
                    SpeakerScoreInfo::from_speech(&speech)
                } else {
                    SpeakerScoreInfo::Hidden
                },
            })
        }
    );
    let participant_gov_debates = participant_gov_debates.into_iter().map(
        |(d, v)| {
            let speech = ballot_map.get(&d.ballot_id).unwrap().speeches.iter().find(|s| s.speaker == Some(participant_id));
            let show_scores = show_results_map.get(&d.round_id).unwrap();
            let speech_score_info = if *show_scores {
                if let Some(speech) = speech {
                    SpeakerScoreInfo::from_speech(&speech)
                }
                else {
                    SpeakerScoreInfo::DidNotParticipate
                }
            }
            else {
                SpeakerScoreInfo::Hidden
            };
            let team_score = match show_scores {
                true => TeamScoreInfo::from_ballot_and_team_role(&ballot_map.get(&d.ballot_id).unwrap(), RoundTeamRole::Government),
                false => TeamScoreInfo::Hidden
            };
            (d.round_id, ParticipantRoundRoleInfo::TeamSpeaker{
                debate: ParticipantDebateInfo::new_from(d, v),
                team_role: RoundTeamRole::Government,
                speaker_score: speech_score_info,
                team_score
            })
        }
    );
    let participant_opp_debates = participant_opp_debates.into_iter().map(
        |(d, v)| {
            let speech = ballot_map.get(&d.ballot_id).unwrap().speeches.iter().find(|s| s.speaker == Some(participant_id));
            let show_scores = show_results_map.get(&d.round_id).unwrap();
            let speech_score_info = if *show_scores {
                if let Some(speech) = speech {
                    SpeakerScoreInfo::from_speech(&speech)
                }
                else {
                    SpeakerScoreInfo::DidNotParticipate
                }
            }
            else {
                SpeakerScoreInfo::Hidden
            };
            let team_score = match show_scores {
                true => TeamScoreInfo::from_ballot_and_team_role(&ballot_map.get(&d.ballot_id).unwrap(), RoundTeamRole::Opposition),
                false => TeamScoreInfo::Hidden
            };

            (d.round_id, ParticipantRoundRoleInfo::TeamSpeaker{
                debate: ParticipantDebateInfo::new_from(d, v),
                team_role: RoundTeamRole::Opposition,
                speaker_score: speech_score_info,
                team_score
            })
        }
    );

    let round_roles : HashMap<Uuid, Vec<ParticipantRoundRoleInfo>> = participant_adjudicator_debates.chain(participant_non_aligned_speaker_debates).chain(participant_gov_debates).chain(participant_opp_debates).into_grouping_map().collect();

    let mut expected_reload = None;

    let rounds = all_rounds.into_iter().map(
        |round| {
            let role = match round_roles.get(&round.uuid) {
                Some(roles) => {
                    if roles.len() == 1 {
                        roles[0].clone()
                    } else {
                        ParticipantRoundRoleInfo::Multiple
                    }
                },
                None => ParticipantRoundRoleInfo::NotDrawn
            };

            let show_motion = check_release_date(current_time, round.full_motion_release_time) || match &role {
                ParticipantRoundRoleInfo::Adjudicator{..} | ParticipantRoundRoleInfo::TeamSpeaker{..} | ParticipantRoundRoleInfo::President {..} => 
                check_release_date(current_time, round.team_motion_release_time),
                ParticipantRoundRoleInfo::NonAlignedSpeaker{debate, ..} => check_release_date(current_time, round.debate_start_time) && debate.is_motion_released_to_non_aligned,
                ParticipantRoundRoleInfo::NotDrawn | ParticipantRoundRoleInfo::Multiple => false
            };

            let next_reload_time = match &role {
                ParticipantRoundRoleInfo::Adjudicator{..} | ParticipantRoundRoleInfo::TeamSpeaker{..} | ParticipantRoundRoleInfo::President {..} => {
                    vec![round.draw_release_time, round.team_motion_release_time, round.debate_start_time, round.round_close_time]
                }
                ParticipantRoundRoleInfo::NonAlignedSpeaker{ ..} => {
                    vec![round.draw_release_time, round.full_motion_release_time, round.debate_start_time, round.round_close_time]
                }
                ParticipantRoundRoleInfo::NotDrawn | ParticipantRoundRoleInfo::Multiple => vec![]
            }.iter().filter(|t| {
                if let Some(t) = t {
                    t > &current_time
                }
                else {
                    false
                }
            }).min().cloned().flatten();

            if let Some(next_reload_time) = next_reload_time {
                if expected_reload.is_none() || Some(next_reload_time) < expected_reload {
                    expected_reload = Some(next_reload_time);
                }
            }

            let status = if check_release_date(current_time, round.round_close_time) {
                RoundStatus::Completed
            } else if check_release_date(current_time, round.debate_start_time) {
                RoundStatus::InProgress
            }
            else if check_release_date(current_time, round.team_motion_release_time) {
                RoundStatus::WaitingToStart { debate_start_time: round.debate_start_time }
            } else if check_release_date(current_time, round.draw_release_time) {
                RoundStatus::DrawReleased {}
            } else {
                RoundStatus::Planned
            };

            ParticipantRoundInfo {
                uuid: round.uuid,
                index: round.index,
                name: format!("Round {}", round.index + 1),
                participant_role: if check_release_date(current_time, round.draw_release_time) { Some(role) } else { None },
                motion: if show_motion {
                    Motion::Shown{motion: round.motion.unwrap_or("<Missing Motion>".into()), info_slide: round.info_slide}
                } else {
                    Motion::Hidden
                },
                status,
                is_silent: round.is_silent,
            }
        }
    ).sorted_by_key(|info| info.index).collect_vec();

    let feedback_requests_debates = rounds.iter().filter_map(|round_info| {
        let show_feedback = match round_info.status {
            RoundStatus::Planned => false,
            RoundStatus::DrawReleased {..} => false,
            RoundStatus::WaitingToStart {..} => false,
            RoundStatus::InProgress => true,
            RoundStatus::Completed => true,
        };
        if !show_feedback {
            return None;
        }
        match &round_info.participant_role {
            Some(ParticipantRoundRoleInfo::Adjudicator { debate, position }) => {
                if *position == 0 {
                    Some((FeedbackSourceRole::Chair, debate.clone(), &round_info.name, &debate.round_id))
                }
                else {
                    Some((FeedbackSourceRole::Wing, debate.clone(), &round_info.name, &debate.round_id))
                }
            },
            Some(ParticipantRoundRoleInfo::NonAlignedSpeaker { debate, .. }) if !round_info.is_silent  => {
                Some((FeedbackSourceRole::NonAligned, debate.clone(), &round_info.name, &debate.round_id))
            },
            Some(ParticipantRoundRoleInfo::TeamSpeaker { debate, .. }) if !round_info.is_silent => {
                Some((FeedbackSourceRole::Team, debate.clone(), &round_info.name, &debate.round_id))
            },
            _ => None,
        }
    }).collect_vec();

    let feedback_directions = get_tournament_feedback_directions(&transaction, participant.tournament_id).await?;

    let mut submission_filter = schema::feedback_response::Column::SourceParticipantId.eq(participant_id);

    if let ParticipantRoleInfo::Speaker { team_id, .. } = &role {
        submission_filter = submission_filter.or(schema::feedback_response::Column::SourceTeamId.eq(*team_id));
    }

    let relevant_submissions = schema::feedback_response::Entity::find()
    .filter(
        submission_filter
    ).all(&transaction).await.map_err(handle_error)?;

    let relevant_submission_map: HashMap<(Uuid, Uuid), Vec<schema::feedback_response::Model>> = relevant_submissions.into_iter().map(|submission| {
        ((submission.source_debate_id, submission.target_participant_id), submission)
    }).into_grouping_map().collect();

    let target_participant_uuids = feedback_requests_debates.iter().flat_map(
        |(request_source_role, debate_info, _round_name, _round_id)| {
            let ballot = ballot_map.get(&debate_info.ballot_id).unwrap();

            let mut out = vec![];

            if feedback_directions.contains(&(*request_source_role, FeedbackTargetRole::Chair)) && ballot.adjudicators.len() > 0 {
                out.push(ballot.adjudicators[0]);
            }
            if feedback_directions.contains(&(*request_source_role, FeedbackTargetRole::Wing)) {
                out.extend(ballot.adjudicators[1..].iter());
            }
            if feedback_directions.contains(&(*request_source_role, FeedbackTargetRole::President)) {
                out.extend(ballot.president.iter());
            }

            out
        }
    ).collect_vec();

    let relevant_names : Vec<(Uuid, String)> = schema::participant::Entity::find().select_only()
    .column(schema::participant::Column::Uuid)
    .column(schema::participant::Column::Name)
    .filter(
        schema::participant::Column::Uuid.is_in(target_participant_uuids)
    ).into_tuple().all(&transaction).await.map_err(handle_error)?;

    let relevant_names = relevant_names.into_iter().collect::<HashMap<_, _>>();
    
    let feedback_requests = feedback_requests_debates.into_iter().flat_map(
        |(request_source_role, debate_info, round_name, round_id)| {
            let ballot = ballot_map.get(&debate_info.ballot_id).unwrap();

            let mut out = vec![];

            let empty_vec = vec![];

            let source_id = match request_source_role {
                FeedbackSourceRole::Chair | FeedbackSourceRole::Wing | FeedbackSourceRole::President | FeedbackSourceRole::NonAligned  => SourceId::Participant { uuid: participant_id },
                FeedbackSourceRole::Team => if let ParticipantRoleInfo::Speaker { team_id, .. } = &role {
                    SourceId::Team { uuid: *team_id }
                } else {
                    // This should be prevented by consistency rules
                    panic!("Participant has team role, but does not have speaker info");
                }
            };

            if feedback_directions.contains(&(request_source_role, FeedbackTargetRole::Chair)) && ballot.adjudicators.len() > 0 {
                let submissions = relevant_submission_map.get(
                    &(
                        debate_info.uuid,
                        ballot.adjudicators[0]
                    )
                ).unwrap_or(&empty_vec);
                out.push(FeedbackSubmissionInfo {
                    source_role: request_source_role,
                    target_role: FeedbackTargetRole::Chair,
                    target_id: ballot.adjudicators[0],
                    target_name: relevant_names.get(&ballot.adjudicators[0]).expect("Missing name").clone(),
                    round_name: round_name.clone(),
                    round_id: round_id.clone(),
                    debate_id: debate_info.uuid,
                    submitted_responses: submissions.iter().map(|s| s.uuid).collect(),
                    source_id
                });
            }
            if feedback_directions.contains(&(request_source_role, FeedbackTargetRole::Wing)) {
                out.extend(ballot.adjudicators[1..].iter().map(|adjudicator| {
                    let submissions = relevant_submission_map.get(
                        &(
                            debate_info.uuid,
                            *adjudicator
                        )
                    ).unwrap_or(&empty_vec);    
                    FeedbackSubmissionInfo {
                        source_role: request_source_role,
                        target_role: FeedbackTargetRole::Wing,
                        target_id: *adjudicator,
                        target_name: relevant_names.get(&adjudicator).expect("Missing name").clone(),
                        round_name: round_name.clone(),
                        round_id: round_id.clone(),
                        debate_id: debate_info.uuid,
                        submitted_responses: submissions.iter().map(|s| s.uuid).collect(),
                        source_id
                    }
                }));
            }
            if feedback_directions.contains(&(request_source_role, FeedbackTargetRole::President)) {
                out.extend(
                    ballot.president.iter().map(|pres| {
                        let submissions = relevant_submission_map.get(
                            &(
                                debate_info.uuid,
                                *pres
                            )
                        ).unwrap_or(&empty_vec);
                        FeedbackSubmissionInfo {
                            source_role: request_source_role,
                            target_role: FeedbackTargetRole::President,
                            target_id: *pres,
                            target_name: relevant_names.get(&pres).expect("Missing name").clone(),
                            round_name: round_name.clone(),
                            round_id: round_id.clone(),
                            debate_id: debate_info.uuid,
                            submitted_responses: submissions.iter().map(|s| s.uuid).collect(),
                            source_id
                        }
        
                    })
                )
            }

            out
        }
    ).collect_vec();

    let published_tournament = open_tab_entities::schema::published_tournament::Entity::find()
    .filter(
        open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament.uuid)
    ).one(&transaction).await.map_err(handle_error)?;

    transaction.rollback().await.map_err(handle_error)?;
    Ok(Json(ParticipantInfoResponse {
        name: participant.name,
        tournament_name: published_tournament.map(|t| t.public_name).unwrap_or(tournament.name),
        role,
        rounds,
        feedback_submissions: feedback_requests,
        expected_reload
    }))
}

async fn get_participant_role(participant_id: Uuid, transaction: &sea_orm::DatabaseTransaction) -> Result<ParticipantRoleInfo, APIError> {
    let speaker_info = open_tab_entities::schema::speaker::Entity::find()
    .find_also_related(open_tab_entities::schema::team::Entity)
    .filter(
        open_tab_entities::schema::speaker::Column::Uuid.eq(participant_id)
    ).one(transaction).await.map_err(handle_error)?;
    let speaker_info = speaker_info.map(
        |(model, team)| {
            let team = team.unwrap(); // Guaranteed by consistency constraints
            (model, team)
        }
    );
    let adjudicator_info = open_tab_entities::schema::adjudicator::Entity::find()
    .filter(
        open_tab_entities::schema::adjudicator::Column::Uuid.eq(participant_id)
    ).one(transaction).await.map_err(handle_error)?;
    let role = match (&speaker_info, &adjudicator_info) {
        (None, None) => ParticipantRoleInfo::None,
        (None, Some(_)) => ParticipantRoleInfo::Adjudicator,
        (Some((_speaker_info, team_info)), None) => {
            ParticipantRoleInfo::Speaker { team_name: team_info.name.clone(), team_id: team_info.uuid }
        },
        (Some(_), Some(_)) => {
            ParticipantRoleInfo::Multiple
        }
    };
    Ok(role)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParticipantShortInfoResponse {
    name: String,
    tournament_name: String,
    role: ParticipantRoleInfo,
    can_edit_clashes: bool
}

async fn get_participant_short_info(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<ParticipantShortInfoResponse>, APIError> {
    if !user.check_is_authorized_as_participant(&db, participant_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view this participant"));
        return Err(err);
    }

    let transaction = db.begin().await.map_err(handle_error)?;
    let participant = open_tab_entities::schema::participant::Entity::find_by_id(participant_id)
    .find_with_related(open_tab_entities::schema::tournament::Entity)
    .all(&transaction).await.map_err(handle_error)?;

    let participant = participant.into_iter().next();

    if participant.is_none() {
        transaction.rollback().await.map_err(handle_error)?;
        return Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")));
    }
    let (participant, tournament) = participant.unwrap();
    let tournament = tournament.into_iter().next().unwrap(); // Guaranteed by consistency constraints

    let published_tournament = open_tab_entities::schema::published_tournament::Entity::find()
    .filter(
        open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament.uuid)
    ).one(&transaction).await.map_err(handle_error)?;

    let role = get_participant_role(participant_id, &transaction).await?;

    transaction.rollback().await.map_err(handle_error)?;
    Ok(Json(ParticipantShortInfoResponse {
        name: participant.name,
        tournament_name: published_tournament.map(|t| t.public_name).unwrap_or(tournament.name),
        role,
        can_edit_clashes: tournament.allow_self_declared_clashes
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantSettings {
    pub is_anonymous: bool
}

pub async fn get_participant_settings(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<ParticipantSettings>, APIError> {
    if !user.check_is_authorized_as_participant(&db, participant_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view this participant"));
        return Err(err);
    }

    let participant = open_tab_entities::schema::participant::Entity::find_by_id(participant_id).one(&db).await.map_err(handle_error)?;
    if let Some(participant) = participant {
        Ok(Json(ParticipantSettings {
            is_anonymous: participant.is_anonymous
        }))
    }
    else {
        Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")))
    }
}

pub async fn update_participant_settings(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(participant_id): Path<Uuid>,
    Json(new_settings): Json<ParticipantSettings>
) -> Result<(), APIError> {
    if !user.check_is_authorized_as_participant(&db, participant_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view this participant"));
        return Err(err);
    }

    let participant = domain::participant::Participant::try_get(&db, participant_id).await?;
    if let Some(mut participant) = participant {
        let tournament_id = participant.tournament_id;
        let mut entity_group = EntityGroup::new(
            participant.tournament_id
        );
        participant.is_anonymous = new_settings.is_anonymous;
        entity_group.add(
            open_tab_entities::Entity::Participant(participant)
        );
        entity_group.save_all_and_log(&db).await?;
        Ok(())
    }
    else {
        Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")))
    }
}

#[derive(Serialize)]
pub struct ParticipantList {
    teams: Vec<TeamInfo>,
    adjudicators: Vec<ParticipantInfo>,
    institutions: HashMap<Uuid, InstitutionInfo>
}

#[derive(Serialize)]
pub struct TeamInfo {
    uuid: Uuid,
    name: String,
    members: Vec<ParticipantInfo>,
}

#[derive(Serialize)]
pub struct ParticipantInfo {
    uuid: Uuid,
    display_name: String,
    institutions: Vec<Uuid>,
    is_anonymous: bool
}

#[derive(Serialize)]
pub struct InstitutionInfo {
    uuid: Uuid,
    name: String,
}

impl From<open_tab_entities::schema::tournament_institution::Model> for InstitutionInfo {
    fn from(model: open_tab_entities::schema::tournament_institution::Model) -> Self {
        Self {
            uuid: model.uuid,
            name: model.name
        }
    }
}

pub async fn list_participants(
    State(db): State<DatabaseConnection>,
    MaybeExtractAuthenticatedUser(user): MaybeExtractAuthenticatedUser,
    Path(tournament_id): Path<Uuid>,
) -> Result<Json<ParticipantList>, APIError> {
    let tournament = open_tab_entities::schema::published_tournament::Entity::find().filter(
        open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament_id)
    ).one(&db).await.map_err(handle_error)?;

    let mut is_authorized = false;
    if let Some(tournament) = tournament {
        is_authorized = tournament.show_participants
    }

    if !is_authorized {
        if let Some(user) = user {
            if !user.check_is_authorized_in_tournament(&db, tournament_id).await? {
                return Err(APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view participants in this tournament")));
            }
        }
        else {
            return Err(APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view participants in this tournament")));
        }
    }

    let institutions_by_id = open_tab_entities::schema::tournament_institution::Entity::find()
    .filter(
        open_tab_entities::schema::tournament_institution::Column::TournamentId.eq(tournament_id)
    ).all(&db).await.map_err(handle_error)?.into_iter().map(|i| (i.uuid, InstitutionInfo::from(i))).collect::<HashMap<_, _>>();

    let mut participants_by_id = open_tab_entities::schema::participant::Entity::find()
    .find_with_related(open_tab_entities::schema::participant_tournament_institution::Entity)
    .filter(
        open_tab_entities::schema::participant::Column::TournamentId.eq(tournament_id)
    ).all(&db).await.map_err(handle_error)?.into_iter().map(
        |(participant, institutions)| {
            let institutions = institutions.into_iter().map(|i| i.institution_id).collect::<Vec<_>>();
            
            (participant.uuid, ParticipantInfo {
                uuid: participant.uuid,
                display_name: if !participant.is_anonymous { participant.name } else { "Anonymous".into() },
                institutions,
                is_anonymous: participant.is_anonymous
            })
        }
    ).collect::<HashMap<_, _>>();

    let teams = open_tab_entities::schema::team::Entity::find()
    .find_with_related(open_tab_entities::schema::speaker::Entity)
    .filter(
        open_tab_entities::schema::team::Column::TournamentId.eq(tournament_id)
    )
    .order_by_asc(open_tab_entities::schema::team::Column::Name)
    .all(&db).await.map_err(handle_error)?.into_iter().map(
        |(team, speakers)| {
            let members = speakers.into_iter().filter_map(|speaker| {
                participants_by_id.remove(&speaker.uuid)
            }).collect::<Vec<_>>();
            TeamInfo {
                uuid: team.uuid,
                name: team.name,
                members
            }
        }
    ).collect::<Vec<_>>();

    let adjudicators = open_tab_entities::schema::adjudicator::Entity::find()
    .inner_join(open_tab_entities::schema::participant::Entity)
    .filter(open_tab_entities::schema::participant::Column::TournamentId.eq(tournament_id))
    .all(&db).await.map_err(handle_error)?.into_iter().filter_map(
        |adjudicator| {
            participants_by_id.remove(&adjudicator.uuid)
        }
    ).collect::<Vec<_>>();

    Ok(Json(ParticipantList {
        teams,
        adjudicators,
        institutions: institutions_by_id
    }))
}

#[derive(Serialize)]
pub struct ParticipantDeclaredClashList {
    pub declared_clashes: Vec<DeclaredClash>,
}

#[derive(Serialize)]
pub struct DeclaredClash {
    pub uuid: Uuid,
    pub participant_id: Uuid,
    pub participant_name: String,
    pub is_self_declared: bool
}

pub async fn get_participant_declared_clashes(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(participant_id): Path<Uuid>,
) -> Result<Json<ParticipantDeclaredClashList>, APIError> {
    if !user.check_is_authorized_as_participant(&db, participant_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view this participant"));
        return Err(err);
    }

    let participant = schema::participant::Entity::find_by_id(participant_id)
    .find_also_related(schema::tournament::Entity)
    .one(&db).await.map_err(handle_error)?;

    if let Some((participant, Some(tournament))) = participant {
        if !tournament.allow_self_declared_clashes {
            return Err(APIError::from((StatusCode::FORBIDDEN, "Self-declared clashes are not allowed in this tournament")));
        }
    }
    else {
        return Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")));
    }

    let clashes = open_tab_entities::schema::participant_clash::Entity::find()
    .filter(
        open_tab_entities::schema::participant_clash::Column::DeclaringParticipantId.eq(participant_id)
    ).all(&db).await.map_err(handle_error)?;

    let clash_target_ids = clashes.iter().map(|clash| clash.target_participant_id).collect::<Vec<_>>();

    let target_participants_by_id = open_tab_entities::schema::participant::Entity::find()
    .filter(
        open_tab_entities::schema::participant::Column::Uuid.is_in(clash_target_ids)
    ).all(&db).await.map_err(handle_error)?.into_iter().map(
        |p| {
            (p.uuid, p.name)
        }
    ).collect::<HashMap<_, _>>();

    let declared_clashes = clashes.into_iter().map(
        |clash| {
            DeclaredClash {
                uuid: clash.uuid,
                participant_id: clash.target_participant_id,
                participant_name: target_participants_by_id.get(&clash.target_participant_id).expect("Missing name").clone(),
                is_self_declared: clash.is_user_declared
            }
        }
    ).collect::<Vec<_>>();

    Ok(Json(ParticipantDeclaredClashList {
        declared_clashes
    }))
}

#[derive(Deserialize)]
pub struct UpdateParticipantClashesRequest {
    #[serde(default)]
    pub added_clashes: Vec<Uuid>,
    #[serde(default)]
    pub removed_clashes: Vec<Uuid>
}

pub async fn update_participant_clashes(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(participant_id): Path<Uuid>,
    Json(request): Json<UpdateParticipantClashesRequest>
) -> Result<(), APIError> {
    if !user.check_is_authorized_as_participant(&db, participant_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view this participant"));
        return Err(err);
    }

    let db = db.begin().await.map_err(handle_error)?;

    let participant = schema::participant::Entity::find_by_id(participant_id)
        .find_also_related(schema::tournament::Entity)
        .one(&db).await.map_err(handle_error)?;
    
    if let Some((participant, Some(tournament))) = participant {
        if !tournament.allow_self_declared_clashes {
            db.rollback().await.map_err(handle_error)?;
            return Err(APIError::from((StatusCode::FORBIDDEN, "Self-declared clashes are not allowed in this tournament")));
        }
        let mut entity_group = EntityGroup::new(
            participant.tournament_id
        );
        let existing_participant_clashes = open_tab_entities::schema::participant_clash::Entity::find()
        .filter(
            open_tab_entities::schema::participant_clash::Column::DeclaringParticipantId.eq(participant_id)
        ).all(&db).await.map_err(handle_error)?.into_iter().map(|clash| (clash.target_participant_id, clash)).into_group_map();
        dbg!(&request.added_clashes);

        for added_clash in request.added_clashes {
            if !existing_participant_clashes.contains_key(&added_clash) {
                entity_group.add(
                    open_tab_entities::Entity::ParticipantClash(open_tab_entities::domain::participant_clash::ParticipantClash {
                        uuid: Uuid::new_v4(),
                        declaring_participant_id: participant_id,
                        target_participant_id: added_clash,
                        clash_severity: 100,
                        was_seen: false,
                        is_approved: false,
                        is_user_declared: true
                    })
                );
            }
        }


        for removed_clash in request.removed_clashes {
            if let Some(clashes) = existing_participant_clashes.get(&removed_clash) {
                for clash in clashes {
                    if clash.is_user_declared {
                        entity_group.delete(
                            open_tab_entities::EntityTypeId::ParticipantClash,
                            clash.uuid
                        );    
                    }
                }
            }
        }

        entity_group.save_all_and_log(&db).await?;

        db.commit().await.map_err(handle_error)?;
        Ok(())
    }
    else {
        Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")))
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
    .route("/tournament/:tournament_id/participants", get(list_participants))
    .route("/participant/:participant_id", get(get_participant_info))
    .route("/participant/:participant_id/info", get(get_participant_short_info))
    .route("/participant/:participant_id/settings", get(get_participant_settings))
    .route("/participant/:participant_id/settings", post(update_participant_settings))
    .route("/participant/:participant_id/clashes", get(get_participant_declared_clashes))
    .route("/participant/:participant_id/clashes", post(update_participant_clashes))
}
