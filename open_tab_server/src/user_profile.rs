use std::sync::Arc;

use axum::Router;
use chrono::{NaiveDate, NaiveDateTime};
use itertools::Itertools;
use open_tab_entities::schema::{tournament, user};
use sea_orm::{prelude::Uuid, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait};
use tokio::sync::Mutex;

use crate::{cache::{self, CacheManager}, state::AppState};

struct UserStatistics {
    tournament_statistics: Vec<UserTournamentStatistic>,
    lifetime_max_speech_score: f64,
    lifetime_max_team_score: f64,
    lifetime_average_speech_score: f64,
    lifetime_average_team_score: f64,

    score_samples: Vec<ScoreSample>,
}

struct ScoreSample {
    total_score: f64,
    time: NaiveDateTime,
}

struct UserTournamentStatistic {
    uuid: Uuid,
    name: String,
    role: UserTournamentRole,
    date: Option<NaiveDateTime>
}

enum UserTournamentRole {
    Speaker {
        average_score: f64,
        speaker_tab_position: i32,
        team_tab_position: i32,
    },
    Adjudicator {
        highest_breaking_round_name: Option<String>,
    },
}

async fn retrieve_user_statistics(
    db: &DatabaseConnection,
    cache: Arc<CacheManager>,
    user_id: Uuid,
    public_only: bool,
) -> anyhow::Result<UserStatistics> {
    let tournaments = if public_only {
        todo!()
    } else {
        tournament::Entity::find()
            .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::participant::Relation::Tournament.def().rev())
            .join(sea_orm::JoinType::InnerJoin, open_tab_entities::schema::user_participant::Relation::Participant.def().rev())
            .filter(open_tab_entities::schema::user_participant::Column::UserId.eq(user_id))
            .order_by_desc(tournament::Column::LastModified)
            .all(db).await?.into_iter().map(
                |model| {
                    (
                        model.uuid,
                        model.name,
                        model.last_modified
                    )
                }
            ).collect_vec()
    };

    for (tournament_id, tournament_name, tournament_date) in tournaments {
        let tournament_tab = cache.get_tab(
            tournament_id,
            vec![],
            false,
            db
        ).await?;
    }

    todo!();
}

pub fn router() -> Router<AppState> {
    Router::new()
}