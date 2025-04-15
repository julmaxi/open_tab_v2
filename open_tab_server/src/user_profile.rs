use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, routing::get, Json, Router};
use chrono::{NaiveDate, NaiveDateTime};
use itertools::Itertools;
use open_tab_entities::{prelude::SpeechRole, schema::{adjudicator, participant, speaker, tournament, tournament_break, tournament_break_speaker, tournament_break_team, tournament_round, user}, tab::TeamRoundRole};
use password_hash::rand_core::le;
use sea_orm::{prelude::Uuid, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Statement};
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

    awards: Vec<AwardInfo>,
}

#[derive(Debug, Serialize)]
struct AwardInfo {
    title: String,
    award_role: AwardRole,
    series_key: Option<String>,
    tournament_id: Uuid,
    tournament_name: String,
    image: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum AwardRole {
    Team,
    Speaker,
    Adjudicator
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
    
    let awards = r#"
SELECT
    awards.participant_id AS participant_id,
    awards.break_award_title AS break_award_title,
    awards.break_award_prestige AS break_award_prestige,
    awards.award_series_key AS award_series_key,
    awards.type AS type,
    awards.tournament_id AS tournament_id,
    tournament.name AS tournament_name,
    award_series.image AS image
    FROM (
    SELECT 
        tournament_break_speaker.speaker_id AS participant_id, 
        tournament_break.break_award_title, 
        tournament_break.break_award_prestige, 
        tournament_break.award_series_key, 
        "s" AS type,
        tournament_break.tournament_id AS tournament_id
    FROM tournament_break
    JOIN tournament_break_speaker 
        ON tournament_break.uuid = tournament_break_speaker.tournament_break_id

    UNION ALL

    SELECT 
        speaker.uuid AS participant_id, 
        tournament_break.break_award_title, 
        tournament_break.break_award_prestige, 
        tournament_break.award_series_key, 
        "t" AS type,
        tournament_break.tournament_id AS tournament_id
    FROM tournament_break
    JOIN tournament_break_team 
        ON tournament_break.uuid = tournament_break_team.tournament_break_id
    JOIN speaker 
        ON tournament_break_team.team_id = speaker.team_id

    UNION ALL

    SELECT 
        tournament_break_adjudicator.adjudicator_id AS participant_id, 
        tournament_break.break_award_title, 
        tournament_break.break_award_prestige, 
        tournament_break.award_series_key, 
        "a" AS type,
        tournament_break.tournament_id AS tournament_id
    FROM tournament_break
    JOIN tournament_break_adjudicator 
        ON tournament_break.uuid = tournament_break_adjudicator.tournament_break_id
) AS awards
JOIN user_participant 
    ON user_participant.participant_id = awards.participant_id
JOIN tournament
    ON tournament.uuid = awards.tournament_id
LEFT JOIN award_series ON award_series.short_name = awards.award_series_key
WHERE user_participant.user_id = $1 AND awards.break_award_title IS NOT NULL
ORDER BY awards.break_award_prestige DESC;
    "#;

    let awards  : Result<Vec<_>, _> = open_tab_entities::schema::tournament_break::Entity::find()
        .select_only()
        .from_raw_sql(Statement::from_sql_and_values(
            db.get_database_backend(),
            awards,
            vec![user_id.into()],
        ))
        .into_json()
        .all(db).await?.into_iter().map(
            |values| {
                let award_role_str = values.get("type")
                .ok_or(anyhow::Error::msg("Invalid award role"))?;

                let serde_json::Value::String(award_role_str) = award_role_str else {
                    return Err(anyhow::Error::msg("Invalid award role"));
                };
    
                let award_role = match award_role_str.as_str() {
                    "s" => Ok(AwardRole::Speaker),
                    "t" => Ok(AwardRole::Team),
                    "a" => Ok(AwardRole::Adjudicator),
                    _ => Err(anyhow::Error::msg("Invalid award role"))
                }?;

                let award_series_key = values.get("award_series_key");

                let award_series_key = if let Some(serde_json::Value::String(award_series_key)) = award_series_key {
                    Some(award_series_key.clone())
                } else {
                    None
                };

                let award_prestige = values.get("break_award_prestige");

                let award_prestige = if let Some(serde_json::Value::Number(award_prestige)) = award_prestige {
                    Some(award_prestige.as_i64().unwrap_or_default() as i32)
                } else {
                    None
                };

                let award_title = values.get("break_award_title");
                let award_title = if let Some(serde_json::Value::String(award_title)) = award_title {
                    award_title.clone()
                } else {
                    return Err(anyhow::Error::msg("Invalid award title"))
                };

                let tournament_id = values.get("tournament_id")
                    .ok_or(anyhow::Error::msg("Invalid tournament ID"))?;
                let serde_json::Value::String(tournament_id) = tournament_id else {
                    return Err(anyhow::Error::msg("Invalid tournament ID"));
                };
                let tournament_id = Uuid::parse_str(tournament_id)?;

                let tournament_name = values.get("tournament_name")
                    .ok_or(anyhow::Error::msg("Invalid tournament name"))?;
                let serde_json::Value::String(tournament_name) = tournament_name else {
                    return Err(anyhow::Error::msg("Invalid tournament name"));
                };

                let image = if let Some(serde_json::Value::String(image)) = values.get("image") {
                    Some(image.clone())
                }
                else {
                    None
                };

                Ok(AwardInfo {
                    title: award_title,
                    series_key: award_series_key,
                    award_role,
                    tournament_id,
                    tournament_name: tournament_name.clone(),
                    image
                })
            }
        ).collect();

    let awards = awards?;

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
                    for (idx, _round_id) in tab.rounds.iter().enumerate() {
                        if let Some(Some(score)) = speaker_entry.detailed_scores.get(idx) {
                            let score_val = score.score;
                            speaker_max_score = score_val.max(speaker_max_score);
                            speaker_score_sum += score_val;
                            speaker_score_count += 1;
                            if let Some(round_time) = round_times.get(idx) {
                                score_samples.push(ScoreSample {
                                    total_score: score_val,
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
            },
            awards
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