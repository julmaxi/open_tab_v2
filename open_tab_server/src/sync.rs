use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::{extract::{Query, Path, State}, Router, routing::{get, post}, Json};
use chrono::Utc;
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{get_changed_entities_from_log, Entity, EntityGroup, EntityState, EntityTypeId, EntityTypeIdTrait, NewEntityState};
use sea_orm::{prelude::*, DatabaseConnection, QueryOrder, TransactionTrait, IntoActiveModel, QuerySelect};
use serde::{Deserialize, Serialize};

use tokio::sync::RwLock;
use tracing::error_span;

use crate::{state::AppState, response::APIError, auth::ExtractAuthenticatedUser};




#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityEntry<E, T> {
    pub uuid: Uuid,
    pub old_versions: Vec<Uuid>,
    pub current_version: Uuid,
    pub current_value: EntityState<E, T>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry<T> {
    pub uuid: Uuid,
    pub target_type: T,
    pub target_uuid: Uuid,
    pub timestamp: DateTime,
}

impl <T>From<&open_tab_entities::schema::tournament_log::Model> for LogEntry<T> where T: EntityTypeIdTrait {
    fn from(model: &open_tab_entities::schema::tournament_log::Model) -> Self {
        LogEntry {
            uuid: model.uuid,
            target_type: model.target_type.clone().into(),
            target_uuid: model.target_uuid,
            timestamp: model.timestamp,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatLog<E, T> where T: EntityTypeIdTrait {
    pub log: Vec<LogEntry<T>>,
    pub entities: HashMap<
        T,
        Vec<
            EntityEntry<E, T>
        >
    >
}

pub async fn get_log_since<C>(transaction: &C, tournament_id: Uuid, since: Option<Uuid>) -> Result<Vec<open_tab_entities::schema::tournament_log::Model>, anyhow::Error> where C: sea_orm::ConnectionTrait  {
    let log_query: Select<open_tab_entities::schema::tournament_log::Entity> = open_tab_entities::schema::tournament_log::Entity::find()
        .filter(open_tab_entities::schema::tournament_log::Column::TournamentId.eq(tournament_id))
        .order_by_asc(open_tab_entities::schema::tournament_log::Column::SequenceIdx);

    let log_query = match since {
        None => log_query,
        Some(since) => {
            let model = open_tab_entities::schema::tournament_log::Entity::find_by_id(since).one(transaction).await?.ok_or(anyhow::anyhow!("Since is not a valid log entry"))?;
            log_query.filter(open_tab_entities::schema::tournament_log::Column::SequenceIdx.gt(model.sequence_idx))
        }
    };

    let log = log_query
        .all(transaction)
        .await?;
    Ok(log)
}


pub async fn get_entity_changes_since<C>(transaction: &C, tournament_id: Uuid, since: Option<Uuid>) -> Result<FatLog<Entity, EntityTypeId>, anyhow::Error>
    where C: sea_orm::ConnectionTrait  {
    let log = get_log_since(transaction, tournament_id, since).await?;
    let flat_log = log.iter().map(
        LogEntry::from
    ).collect::<Vec<LogEntry<EntityTypeId>>>();
    let mut entity_entries : HashMap<(EntityTypeId, _), _> = log
        .into_iter()
        .into_grouping_map_by(|entry| (entry.target_type.clone().into(), entry.target_uuid)
    ).collect::<Vec<_>>();
    let latest_entries = entity_entries.iter_mut().map(|((_entity_type, _entity_uuid), entries)| {
        entries.pop().unwrap() // This can never be empty, otherwise the key would not be in the group map
    }).collect::<Vec<_>>();
    let versioned_entities = get_changed_entities_from_log(transaction, latest_entries).await?;

    let versioned_entities_map = versioned_entities.into_iter().map(|entity| {
        ((
            entity.entity.get_type(),
            entity.entity.get_uuid()
        ), entity)
    }).collect::<HashMap<_, _>>();

    let entities = entity_entries.into_iter().map(|((entity_type, entity_uuid), entries)| {
        let latest_version = versioned_entities_map.get(&(entity_type.clone(), entity_uuid)).unwrap();
        (entity_type, EntityEntry {
            uuid: entity_uuid,
            old_versions: entries.into_iter().map(|entry| entry.target_uuid).collect::<Vec<_>>(),
            current_version: latest_version.version,
            current_value: latest_version.entity.clone().into()
        })
    }).into_grouping_map().collect::<Vec<_>>();

    Ok(FatLog {
        log: flat_log,
        entities
    })
}

async fn get_log(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(tournament_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>
) -> Result<Json<FatLog<Entity, EntityTypeId>>, APIError> {
    let tournament = open_tab_entities::schema::tournament::Entity::find_by_id(tournament_id).one(&db).await?;
    if tournament.is_none() {
        return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Tournament not found"));
    }
    if !user.check_is_authorized_for_tournament_administration(&db, tournament_id).await? {
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not authorized for tournament administration"));
    }

    let since = match params.get("since").map(
        |since| since.parse::<Uuid>()
    ) {
        None => Ok(None),
        Some(Err(_)) => {
            error_span!("since must be an uuid");
            Err(APIError::new("since must be an uuid".into()))
        },
        Some(Ok(since)) => Ok(Some(since))
    }?;

    let transaction = db.begin().await.map_err(|_| {
        error_span!("Failed to start transaction");
        APIError::new("Failed to start transaction".into())
    })?;
    let fat_log = get_entity_changes_since(&transaction, tournament_id, since).await?;
    transaction.rollback().await?;
    Ok(Json(
        fat_log
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeStrategy {
    Reject,
    AlwaysLocal
}

pub enum ReconciliationOutcome {
    Reject,
    InvalidTournament,
    Success {new_last_common_ancestor: Uuid, entity_group: Option<EntityGroup>}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIReconciliationOutcome {
    Reject,
    InvalidTournament,
    Success {new_last_common_ancestor: Uuid}
}

impl From<ReconciliationOutcome> for APIReconciliationOutcome {
    fn from(outcome: ReconciliationOutcome) -> Self {
        match outcome {
            ReconciliationOutcome::Reject => APIReconciliationOutcome::Reject,
            ReconciliationOutcome::InvalidTournament => APIReconciliationOutcome::InvalidTournament,
            ReconciliationOutcome::Success {new_last_common_ancestor, entity_group: _} => APIReconciliationOutcome::Success {
                new_last_common_ancestor,
            }
        }
    }
}



pub async fn reconcile_changes<C>(
    db: &C,
    tournament_id: Uuid,
    changes: FatLog<Entity, EntityTypeId>,
    last_common_ancestor: Option<Uuid>,
    merge_strategy: MergeStrategy,
    return_entity_group: bool
) -> Result<ReconciliationOutcome, anyhow::Error> where C: sea_orm::ConnectionTrait {
    let local_log = get_log_since(db, tournament_id, last_common_ancestor).await?;

    if last_common_ancestor.is_none() && changes.log.len() == 0 {
        // We can not properly return the last common ancestor when the log is empty,
        // so we must reject this special case
        tracing::warn!("Rejecting reconciliation with empty log and no last common ancestor");
        return Ok(ReconciliationOutcome::Reject);
    }

    if local_log.len() > 0 && merge_strategy == MergeStrategy::Reject {
        tracing::warn!("Rejecting reconciliation with local changes and {} local log entries", local_log.len());
        return Ok(ReconciliationOutcome::Reject);
    }

    let head_sequence_idx = if local_log.len() > 0 {
        local_log.last().map(|entry| entry.sequence_idx).unwrap()
    }
    else {
        open_tab_entities::schema::tournament_log::Entity::find()
        .filter(open_tab_entities::schema::tournament_log::Column::TournamentId.eq(tournament_id))
        .order_by_desc(open_tab_entities::schema::tournament_log::Column::SequenceIdx)
        .limit(1)
        .one(db).await?.map(|m| m.sequence_idx).unwrap_or(0)
    };
    let locally_changed_entities = local_log.iter().map(|entry| (entry.target_type.clone().into(), entry.target_uuid, entry.uuid)).collect::<HashSet<_>>();

    let existing_entries = local_log.iter().map(|entry| entry.uuid).collect::<HashSet<_>>();

    let mut remote_log_models = changes.log.iter().filter(|entry| !existing_entries.contains(&entry.uuid) ).enumerate().map(
        |(idx, entry)| {
            open_tab_entities::schema::tournament_log::Model {
                uuid: entry.uuid,
                tournament_id,
                target_type: entry.target_type.as_str().into(),
                target_uuid: entry.target_uuid,
                timestamp: entry.timestamp,
                sequence_idx: head_sequence_idx + idx as i32 + 1
            }.into_active_model()
        }
    ).collect_vec();

    if remote_log_models.len() == 0 {
        return Ok(ReconciliationOutcome::Success { new_last_common_ancestor: changes.log.last().unwrap().uuid, entity_group: None })
    }

    // This unwrap is safe, since we reject an empty log with no last common ancestor
    let new_last_common_ancestor = remote_log_models.last().map(|model| model.uuid.clone().unwrap()).unwrap_or_else(|| last_common_ancestor.unwrap());
    let new_head_idx = remote_log_models.last().map(|model| model.sequence_idx.clone().unwrap()).unwrap_or(head_sequence_idx);

    let remote_changes_entities = changes.entities.iter().flat_map(|(entity_type, entries)| {
        entries.iter().map(|entry| (entity_type.clone(), entry.uuid, entry.current_version))
    }).collect::<HashSet<_>>();

    let conflicting_entities = locally_changed_entities.intersection(&remote_changes_entities).collect::<HashSet<_>>();

    conflicting_entities.iter().enumerate().for_each(|(idx, (entity_type, uuid, _version))| {
        remote_log_models.push(open_tab_entities::schema::tournament_log::Model {
            uuid: Uuid::new_v4(),
            tournament_id,
            target_type: entity_type.as_str().into(),
            target_uuid: uuid.clone(),
            timestamp: Utc::now().naive_utc(),
            sequence_idx: new_head_idx + idx as i32 + 1
        }.into_active_model());
    });

    if remote_log_models.len() > 0 {
        dbg!("Inserting remote log models", &remote_log_models.len());
        open_tab_entities::schema::tournament_log::Entity::insert_many(remote_log_models).exec(db).await?;
    }
    
    let conflicting_entities = conflicting_entities.into_iter().map(|(entity_type, uuid, _version)| (entity_type.clone(), uuid.clone())).collect::<HashSet<_>>();
    // We bypass the normal save logic here, since we save the entire log at once
    let mut entities_to_save = vec![];
    for (entity_type, entities) in changes.entities.into_iter() {
        for entry in entities {
            if !conflicting_entities.contains(&(entity_type.clone(), entry.uuid)) {
                entities_to_save.push(entry.current_value);
            }
        }
    }

    //let group = EntityGroup::from(entities_to_save.into_iter());
    let mut group = EntityGroup::new(tournament_id);

    for entity in entities_to_save {
        match entity {
            EntityState::Exists(e) => {
                group.add(e);
            },
            EntityState::Deleted {uuid, type_} => {
                group.delete(type_, uuid);
            }
        }
    }
    group.save_all(db).await?;

    let existing_entities = open_tab_entities::schema::tournament_entity::Entity::find()
        .filter(open_tab_entities::schema::tournament_entity::Column::Uuid.is_in(group.get_all_related_uuids()))
        .all(db).await?;

    for e in existing_entities.iter() {
        if e.tournament_id != tournament_id {
            return Ok(
                ReconciliationOutcome::InvalidTournament
            );
        }
    }

    let existing_entity_map = existing_entities.into_iter().map(|e| (e.uuid, e)).collect::<HashMap<_, _>>();

    let mut new_entities = vec![];

    let mut altered_entities = vec![];

    for ((entity_type, uuid), entity) in group.entity_states.iter() {
        let e = existing_entity_map.get(&uuid);
        let mut did_exist = false;

        let mut e = if let Some(e) = e {
            did_exist = true;
            e.clone().into_active_model()
        } else {
            open_tab_entities::schema::tournament_entity::ActiveModel {
                uuid: sea_orm::ActiveValue::Set(*uuid),
                entity_type: sea_orm::ActiveValue::Set(entity_type.as_str().into()),
                tournament_id: sea_orm::ActiveValue::Set(tournament_id),
                is_deleted: sea_orm::ActiveValue::Set(false)
            }
        };

        match entity {
            NewEntityState::Exists(_) => {
                if !did_exist {
                    new_entities.push(e);
                }
            },
            NewEntityState::Deleted => {
                e.is_deleted = sea_orm::ActiveValue::Set(true);
                if did_exist {
                    altered_entities.push(e);
                }
                else {
                    new_entities.push(e);
                }
            }
        }
    }

    if new_entities.len() > 0 {
        open_tab_entities::schema::tournament_entity::Entity::insert_many(new_entities).exec(db).await?;
    }

    for e in altered_entities {
        e.update(db).await?;
    }

    let tournament = open_tab_entities::schema::tournament::ActiveModel {
        uuid: sea_orm::ActiveValue::Unchanged(tournament_id),
        last_modified: sea_orm::ActiveValue::Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    tournament.update(db).await?;

    /*
    let deleted_tournamet_uuids = group.get_all_deletion_tournaments(db).await?;
    if !deleted_tournamet_uuids.into_iter().all(|t| t == Some(tournament_id)) {
        println!("Rejecting push trying to delete in other tournaments");
        return Ok(
            ReconciliationOutcome::InvalidTournament
        );
    }
    group.save_all(db).await?;

    let added_tournament_uuids = group.get_all_tournaments(db).await?;
    if !added_tournament_uuids.into_iter().all(|t| t == Some(tournament_id)) {
        dbg!("Rejecting push trying to modify in other tournaments");

        return Ok(
            ReconciliationOutcome::InvalidTournament
        );
    }
     */

    Ok(
        ReconciliationOutcome::Success {
            new_last_common_ancestor,
            entity_group: if return_entity_group { Some(group) } else { None }
        }
    )
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest<E, T> where T: EntityTypeIdTrait {
    pub log: FatLog<E, T>,
    pub last_common_ancestor: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestResponse {
    pub outcome: APIReconciliationOutcome
}


async fn handle_sync_push_request(
    State(db): State<DatabaseConnection>,
    State(notifications): State<Arc<RwLock<crate::notify::ParticipantNotificationManager>>>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(tournament_id): Path<Uuid>,
    Json(request_body): Json<SyncRequest<Entity, EntityTypeId>>
) -> Result<Json<SyncRequestResponse>, APIError> {
    let tournament = open_tab_entities::schema::tournament::Entity::find_by_id(tournament_id).one(&db).await?;
    if tournament.is_none() {
        return Err(APIError::new_with_status(StatusCode::NOT_FOUND, "Tournament not found"));
    }

    if !user.check_is_authorized_for_tournament_administration(&db, tournament_id).await? {
        return Err(APIError::new_with_status(StatusCode::FORBIDDEN, "User is not authorized for tournament administration"));
    }

    let transaction = db.begin().await.map_err(|_| {
        tracing::error!("Failed to start transaction");
        APIError::new("Failed to start transaction".into())
    })?;
 
    let outcome = reconcile_changes(
        &transaction,
        tournament_id,
        request_body.log,
        request_body.last_common_ancestor,
        MergeStrategy::Reject,
        true
    ).await?;

    match &outcome {
        ReconciliationOutcome::Reject => {
            transaction.rollback().await?;
            return Ok(
                Json(
                    SyncRequestResponse {
                        outcome: outcome.into()
                    }
                )
            )
        }
        ReconciliationOutcome::InvalidTournament => {
            transaction.rollback().await?;
            return Err(APIError::new_with_status(StatusCode::BAD_REQUEST, "Invalid tournament"));
        },
        ReconciliationOutcome::Success { entity_group, .. } => {
            notifications.read().await.process_entities(&transaction, entity_group.as_ref().unwrap()).await;

            transaction.commit().await?;
            return Ok(
                Json(
                    SyncRequestResponse {
                        outcome: outcome.into()
                    }
                )
            )
        },
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
    .route("/tournament/:tournament_id/log", get(get_log))
    .route("/tournament/:tournament_id/log", post(handle_sync_push_request))
}