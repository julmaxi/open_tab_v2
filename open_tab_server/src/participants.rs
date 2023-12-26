use std::collections::HashMap;

use axum::{extract::{Path, State}, Json, Router, routing::{get, post}};
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{domain::{entity::LoadEntity, feedback_form::{FeedbackForm, FeedbackFormVisibility, FeedbackSourceRole, FeedbackTargetRole}, ballot::SpeechRole, self}, schema, EntityGroup, EntityGroupTrait};
use sea_orm::{DatabaseConnection, TransactionTrait, prelude::*, QuerySelect, QueryOrder};
use serde::{Serialize, Deserialize};

use crate::{response::{APIError, handle_error}, auth::ExtractAuthenticatedUser, state::AppState};

use open_tab_entities::domain::round::check_release_date;


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type")]
pub struct ParticipantInfoResponse {
    pub name: String,
    pub role: ParticipantRoleInfo,
    pub rounds: Vec<ParticipantRoundInfo>,
    pub feedback_submissions: Vec<FeedbackSubmissionInfo>
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
    if !user.check_is_authorized_as_participant(&db, participant_id).await? {
        let err = APIError::from((StatusCode::FORBIDDEN, "You are not authorized to view this participant"));
        return Err(err);
    }
    let transaction = db.begin().await.map_err(handle_error)?;

    let participant_query_result = open_tab_entities::schema::participant::Entity::find_by_id(participant_id)
    .find_also_related(open_tab_entities::schema::tournament::Entity)
        .one(&transaction).await.map_err(handle_error)?;

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

    let all_feedback_forms = FeedbackForm::get_all_in_tournament(&transaction, participant.tournament_id).await?;

    let overall_visibility = all_feedback_forms.iter().fold(
        Default::default(),
        |acc : FeedbackFormVisibility, val| {
            acc | &val.visibility
        }
    );

    let feedback_directions = overall_visibility.to_feedback_direction_pairs();

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

            if feedback_directions.contains(&(*request_source_role, FeedbackTargetRole::Chair)) {
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

            if feedback_directions.contains(&(request_source_role, FeedbackTargetRole::Chair)) {
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

    transaction.rollback().await.map_err(handle_error)?;
    Ok(Json(ParticipantInfoResponse {
        name: participant.name,
        role,
        rounds,
        feedback_submissions: feedback_requests
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
    role: ParticipantRoleInfo,
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
    .one(&transaction).await.map_err(handle_error)?;

    if participant.is_none() {
        transaction.rollback().await.map_err(handle_error)?;
        return Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")));
    }
    let participant = participant.unwrap();

    let role = get_participant_role(participant_id, &transaction).await?;

    transaction.rollback().await.map_err(handle_error)?;
    Ok(Json(ParticipantShortInfoResponse {
        name: participant.name,
        role
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
        let mut entity_group = EntityGroup::new();
        participant.is_anonymous = new_settings.is_anonymous;
        entity_group.add(
            open_tab_entities::Entity::Participant(participant)
        );
        entity_group.save_all_and_log_for_tournament(&db, tournament_id).await?;
        Ok(())
    }
    else {
        Err(APIError::from((StatusCode::NOT_FOUND, "Participant not found")))
    }
}


pub fn router() -> Router<AppState> {
    Router::new()
    .route("/participant/:participant_id", get(get_participant_info))
    .route("/participant/:participant_id/info", get(get_participant_short_info))
    .route("/participant/:participant_id/settings", get(get_participant_settings))
    .route("/participant/:participant_id/settings", post(update_participant_settings))
}