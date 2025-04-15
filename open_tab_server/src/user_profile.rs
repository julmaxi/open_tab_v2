use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, routing::get, Json, Router};
use chrono::{NaiveDate, NaiveDateTime};
use itertools::Itertools;
use open_tab_entities::{prelude::SpeechRole, schema::{adjudicator, participant, speaker, tournament, tournament_round, user}, tab::TeamRoundRole};
use password_hash::rand_core::le;
use sea_orm::{prelude::Uuid, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{auth::ExtractAuthenticatedUser, cache::{self, CacheManager}, response::APIError, state::AppState};

#[derive(Debug, Serialize)]
struct UserStatistics {
    tournament_statistics: Vec<UserTournamentStatistic>,
    lifetime_max_speech_score: f64,
    lifetime_max_team_score: f64,
    lifetime_average_speech_score: f64,
    lifetime_average_team_score: f64,
    lifetime_adjudication_ratio: f64,

    score_samples: Vec<ScoreSample>,
}

#[derive(Debug, Serialize)]
struct ScoreSample {
    total_score: f64,
    time: NaiveDateTime,
    role: TeamRoundRole,
    position: u8,
}

#[derive(Debug, Serialize)]
struct UserTournamentStatistic {
    uuid: Uuid,
    name: String,
    role: UserTournamentRole,
    date: NaiveDateTime
}

#[derive(Debug, Serialize)]
#[serde(tag="role")]
enum UserTournamentRole {
    Speaker {
        average_score: f64,
        speaker_tab_position: u32,
        team_tab_position: u32,
    },
    Adjudicator {},
}

async fn retrieve_user_statistics(
    db: &DatabaseConnection,
    cache: Arc<CacheManager>,
    user_id: Uuid,
    public_only: bool,
) -> anyhow::Result<UserStatistics> {
    let now = chrono::Utc::now().naive_utc();
    let tournaments = if public_only {
        todo!()
    } else {
        tournament::Entity::find()
            .select_only()
            .column(tournament::Column::Uuid)
            .column(tournament::Column::Name)
            .column(tournament::Column::LastModified)
            .column(participant::Column::Uuid)
            .column(speaker::Column::TeamId)
            .column(adjudicator::Column::Uuid)
            .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::participant::Relation::Tournament.def().rev())
            .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::user_participant::Relation::Participant.def().rev())
            .join(sea_orm::JoinType::LeftJoin, participant::Relation::Speaker.def())
            .join(sea_orm::JoinType::LeftJoin, participant::Relation::Adjudicator.def())
            .filter(open_tab_entities::schema::user_participant::Column::UserId.eq(user_id))
            .order_by_desc(tournament::Column::LastModified)
            .into_tuple::<(Uuid, String, Option<NaiveDateTime>, Uuid, Option<Uuid>, Option<Uuid>)>()
            .all(db).await?
    };

    let tournament_rounds = tournament_round::Entity::find()
        .select_only()
        .column(tournament_round::Column::Uuid)
        .column(tournament_round::Column::TournamentId)
        .column(tournament_round::Column::RoundCloseTime)
        .filter(tournament_round::Column::TournamentId.is_in(
            tournaments.iter().map(|(tournament_id, _, _, _, _, _)| *tournament_id).collect_vec()
        ))
        .filter(
            tournament_round::Column::RoundCloseTime.lt(now).and(
                tournament_round::Column::IsSilent.eq(false)
            ).or(
                tournament_round::Column::SilentRoundResultsReleaseTime.lt(now)
            )
        )
        .into_tuple::<(Uuid, Uuid, Option<NaiveDateTime>)>()
        .all(db).await?.into_iter().into_group_map_by(|(_, tournament_id, _)| *tournament_id);


    let mut speaker_max_score = 0.0;
    let mut speaker_score_sum = 0.0;
    let mut speaker_score_count = 0;

    let mut team_max_score = 0.0;
    let mut team_score_sum = 0.0;
    let mut team_score_count = 0;
    let mut tournament_statistics = vec![];

    let mut adjudicator_count = 0;
    
    let mut score_samples = vec![];
    
    for (tournament_id, tournament_name, tournament_date, tournament_participant_id, team_id, adjudicator_id) in tournaments {
        if let Some(team_id) = team_id {
            let round_ids = tournament_rounds.get(&tournament_id).iter().map(|i| i.iter()).flatten().map(|(round_id, _, _)| *round_id).collect_vec();
            let round_times = tournament_rounds.get(&tournament_id).iter().map(|i| i.iter()).flatten().map(|(_, _, round_time)| *round_time).collect_vec();
            let tab: open_tab_entities::tab::TabView = cache.get_tab(tournament_id, round_ids, db).await?;
    
            let speaker_entry = tab.speaker_index.get(&tournament_participant_id).and_then(|idx| {
                tab.speaker_tab.get(*idx)
            });
            let team_entry = tab.team_index.get(&team_id).and_then(|idx| {
                tab.team_tab.get(*idx)
            });

            match (speaker_entry, team_entry) {
                (Some(speaker_entry), Some(team_entry)) => {
                    for (idx, round_id) in tab.rounds.iter().enumerate() {
                        if let Some(Some(score)) = speaker_entry.detailed_scores.get(idx) {
                            let scoreVal = score.score;
                            speaker_max_score = scoreVal.max(speaker_max_score);
                            speaker_score_sum += scoreVal;
                            speaker_score_count += 1;
                            if let Some(round_time) = round_times.get(idx) {
                                score_samples.push(ScoreSample {
                                    total_score: scoreVal,
                                    time: round_time.clone().unwrap_or_default(),
                                    role: score.team_role.clone(),
                                    position: score.speech_position
                                });
                            }
                            if let Some(Some(score)) = team_entry.detailed_scores.get(idx) {
                                let score = score.total_score();
                                team_max_score = score.max(team_max_score);
                                team_score_sum += score;
                                team_score_count += 1;
                            }
                        }
                    }
                    tournament_statistics.push(UserTournamentStatistic {
                        uuid: tournament_id,
                        name: tournament_name.clone(),
                        role: UserTournamentRole::Speaker {
                            average_score: speaker_entry.total_score,
                            speaker_tab_position: speaker_entry.rank,
                            team_tab_position: team_entry.rank
                        },
                        date: tournament_date.unwrap_or_default()
                    });
                }
                _ => {}
            }

            if adjudicator_id.is_some() {
                tournament_statistics.push(UserTournamentStatistic {
                    uuid: tournament_id,
                    name: tournament_name.clone(),
                    role: UserTournamentRole::Adjudicator {},
                    date: tournament_date.unwrap_or_default()
                });
                adjudicator_count += 1;
            }
        }
    }

    score_samples.sort_by_key(|sample| sample.time);

    Ok(
        UserStatistics {
            tournament_statistics,
            lifetime_max_speech_score: speaker_max_score,
            lifetime_max_team_score: team_max_score,
            lifetime_average_speech_score: safe_avg(speaker_score_sum, speaker_score_count as f64),
            lifetime_average_team_score: safe_avg(team_score_sum, team_score_count as f64),
            score_samples,
            lifetime_adjudication_ratio: if speaker_score_count > 0 {
                adjudicator_count as f64 / speaker_score_count as f64
            } else {
                0.0
            }
        }
    )
}

fn safe_avg(sum: f64, count: f64) -> f64 {
    if count > 0.0 {
        sum / count
    } else {
        0.0
    }
}

async fn get_user_private_statistics(
    State(state) : State<AppState>,
    Path(user_id) : Path<Uuid>,
    ExtractAuthenticatedUser(user) : ExtractAuthenticatedUser,

) -> Result<Json<UserStatistics>, APIError> {
    let db = state.db.clone();
    let cache = state.cache_manager.clone();
    if user.uuid != user_id {
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "You are not authorized to access this resource"));
    }

    let statistics = retrieve_user_statistics(&db, cache, user_id, false).await?;
    Ok(Json(statistics))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/user/:user_id/stats", get(get_user_private_statistics))
}