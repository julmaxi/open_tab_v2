

use axum::{extract::{Path, State}, Json, Router, routing::get};
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{domain::{self, participant::ParticipantInstitution}, prelude::TournamentRound, schema, tab::{AugmentedTabView, TabView}};
use sea_orm::{prelude::*, sea_query::IntoCondition, DatabaseConnection, DbBackend, QuerySelect, QueryTrait, RelationBuilder};
use serde::{Serialize, Deserialize};
use std::{collections::HashMap, sync::Arc};

use crate::{auth::MaybeExtractAuthenticatedUser, cache::CacheManager, response::APIError, state::AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct TabResponse {
    tab: AugmentedTabView,
    well_known_institutions: HashMap<String, WellKnownInstitutionInfo>,
    participant_well_known_institutions: HashMap<Uuid, Vec<String>>,
    team_well_known_institutions: HashMap<Uuid, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WellKnownInstitutionInfo {
    pub short_name: String,
    pub icon: Option<Uuid>,
}


pub async fn get_current_tab(
    State(state): State<AppState>,
    Path(tournament_id): Path<Uuid>,
    MaybeExtractAuthenticatedUser(user): MaybeExtractAuthenticatedUser,
) -> Result<Json<TabResponse>, APIError> {
    let db = state.db.clone();
    let cache_manager = state.cache_manager.clone();
    let published_tournament = open_tab_entities::schema::published_tournament::Entity::find()
        .filter(open_tab_entities::schema::published_tournament::Column::TournamentId.eq(tournament_id))
        .one(&db)
        .await?;
    
    let allow_unchecked_access = published_tournament.map(|t| t.show_tab).unwrap_or(false);
    if !allow_unchecked_access {
        if let Some(user) = user {
            if !user.check_is_authorized_in_tournament(&db, tournament_id).await? {
                let err = APIError::new_with_status(StatusCode::FORBIDDEN, "You are not authorized for this tournament");
                return Err(err);
            }
        }
        else {
            let err = APIError::new_with_status(StatusCode::UNAUTHORIZED, "You must be logged in to access this tournament");
            return Err(err);
        }
    }
    let tournament_rounds = TournamentRound::get_all_in_tournament(&db, tournament_id).await?;

    let now = chrono::Utc::now().naive_utc();
    let visible_rounds = tournament_rounds.iter().filter(|r| {
        if r.is_silent && !r.silent_round_results_release_time.map_or(false, |t| {
            t <= now
        }) {
            false
        }
        else if r.round_close_time.map_or(false, |t| {
            t <= now
        }) {
            true
        }
        else {
            false
        }
    }).sorted_by_key(|r| r.index).collect_vec();

    //leshow_anonymityt tab = TabView::load_from_tournament_with_rounds_with_anonymity(&db, tournament_id, visible_rounds.iter().map(|r| r.uuid).collect_vec(), true).await?;
    let tab = cache_manager.get_tab(tournament_id, visible_rounds.iter().map(|r| r.uuid).collect_vec(), &db).await?;

    let teams = domain::team::Team::get_all_in_tournament(&db, tournament_id).await?.into_iter().map(|team| (team.uuid, team)).collect::<HashMap<_, _>>();
    let participants = domain::participant::Participant::get_all_in_tournament(&db, tournament_id).await?.into_iter().map(|participant| (participant.uuid, participant)).collect::<HashMap<_, _>>();
    let tab = AugmentedTabView::from_tab_view(&tab, &teams, &participants, true);

    let tournament_well_known_rel = || schema::tournament_institution::Entity::belongs_to(
        schema::well_known_institution::Entity
    ).from(schema::tournament_institution::Column::OfficialIdentifier).to(schema::well_known_institution::Column::ShortName).on_condition(|_left, _right| {
        Expr::col(schema::tournament_institution::Column::OfficialIdentifier)
            .eq(Expr::col(schema::well_known_institution::Column::ShortName)).into_condition()
    }).into();


    let linked_institutions = schema::tournament_institution::Entity::find()
        .select_also(schema::well_known_institution::Entity)
        .join(
            sea_orm::JoinType::InnerJoin,
            tournament_well_known_rel()
        )
        .filter(schema::tournament_institution::Column::TournamentId.eq(tournament_id))
        .all(&db)
        .await?;

    let participant_well_known_institutions = schema::participant::Entity::find()
        .select_only()
        .column(schema::participant::Column::Uuid)
        .column(schema::well_known_institution::Column::ShortName)
        .inner_join(schema::participant_tournament_institution::Entity)
        .join(sea_orm::JoinType::InnerJoin, schema::participant_tournament_institution::Relation::TournamentInstitution.def())
        .join(
            sea_orm::JoinType::InnerJoin,
            tournament_well_known_rel()
        )
        .into_tuple::<(Uuid, String)>()
        .all(&db)
        .await?
        .into_iter()
        .into_group_map();

    let team_well_known_institutions = schema::team::Entity::find()
        .select_only()
        .column(schema::team::Column::Uuid)
        .column(schema::speaker::Column::Uuid)
        .inner_join(schema::speaker::Entity)
        .into_tuple::<(Uuid, Uuid)>()
        .all(&db)
        .await?
        .into_iter()
        .flat_map(|(team_id, speaker_id)| {
            let i = participant_well_known_institutions.get(&speaker_id).map(|v| v.iter()).into_iter().flatten();
            i.map(move |i| {
                (team_id, i.clone())
            })})
        .unique()
        .into_group_map();
    
    return Ok(
        Json(
            TabResponse {
                tab,
                well_known_institutions: linked_institutions.into_iter().filter_map(
                    |(t, i)| {
                        if let Some(i) = i {
                            Some((i.short_name.clone(), WellKnownInstitutionInfo {
                                short_name: i.short_name,
                                icon: i.tiny_image
                            }))
                        }
                        else {
                            None
                        }
                    }
                ).collect(),
                participant_well_known_institutions,
                team_well_known_institutions
            }
        )
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
    .route("/tournament/:tournament_id/tab", get(get_current_tab))
}