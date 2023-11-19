use std::{collections::{HashMap, HashSet}, any};

use axum::{extract::{Query, Path, State}, Router, routing::{get, post}, Json};
use chrono::Utc;
use axum::http::StatusCode;
use itertools::Itertools;
use open_tab_entities::{EntityGroup, Entity, EntityType, get_changed_entities_from_log, EntityGroupTrait, EntityState, EntityTypeId, domain::entity::LoadEntity};
use sea_orm::{prelude::*, DatabaseConnection, QueryOrder, TransactionTrait, IntoActiveModel, Statement, QuerySelect};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::error;
use tracing::error_span;

use crate::{state::AppState, response::{APIError, handle_error}, auth::ExtractAuthenticatedUser};
use std::error::Error;



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

impl <T>From<&open_tab_entities::schema::tournament_log::Model> for LogEntry<T> where T: EntityTypeId {
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
pub struct FatLog<E, T> where T: EntityTypeId {
    pub log: Vec<LogEntry<T>>,
    pub entities: HashMap<
        T,
        Vec<
            EntityEntry<E, T>
        >
    >
}

pub async fn get_log_since<C>(transaction: &C, tournament_id: Uuid, since: Option<Uuid>) -> Result<Vec<open_tab_entities::schema::tournament_log::Model>, anyhow::Error> where C: ConnectionTrait  {
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


pub async fn get_entity_changes_since<C>(transaction: &C, tournament_id: Uuid, since: Option<Uuid>) -> Result<FatLog<Entity, EntityType>, anyhow::Error>
    where C: ConnectionTrait  {
    let log = get_log_since(transaction, tournament_id, since).await?;
    let flat_log = log.iter().map(
        LogEntry::from
    ).collect::<Vec<LogEntry<EntityType>>>();
    let mut entity_entries : HashMap<(EntityType, _), _> = log
        .into_iter()
        .into_grouping_map_by(|entry| (entry.target_type.clone().into(), entry.target_uuid)
    ).collect::<Vec<_>>();
    let latest_entries = entity_entries.iter_mut().map(|((entity_type, entity_uuid), entries)| {
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
) -> Result<Json<FatLog<Entity, EntityType>>, APIError> {
    if !user.check_is_authorized_for_tournament_administration(&db, tournament_id).await? {
        return Err(APIError::from((StatusCode::FORBIDDEN, "User is not authorized for tournament administration")));
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
    transaction.rollback().await.map_err(handle_error)?;
    Ok(Json(
        fat_log
    ))
}

async fn mock<C>(db: &C) -> Result<(), anyhow::Error> where C: ConnectionTrait {
    open_tab_entities::domain::tournament::Tournament::get(db, Uuid::from_u128(1)).await;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeStrategy {
    Reject,
    AlwaysLocal
}

pub enum ReconciliationOutcome {
    Reject,
    Success {new_last_common_ancestor: Uuid, entity_group: Option<EntityGroup>}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum APIReconciliationOutcome {
    Reject,
    Success {new_last_common_ancestor: Uuid}
}

impl From<ReconciliationOutcome> for APIReconciliationOutcome {
    fn from(outcome: ReconciliationOutcome) -> Self {
        match outcome {
            ReconciliationOutcome::Reject => APIReconciliationOutcome::Reject,
            ReconciliationOutcome::Success {new_last_common_ancestor, entity_group} => APIReconciliationOutcome::Success {
                new_last_common_ancestor,
            }
        }
    }
}

pub async fn reconcile_changes<C>(
    db: &C,
    tournament_id: Uuid,
    changes: FatLog<Entity, EntityType>,
    last_common_ancestor: Option<Uuid>,
    merge_strategy: MergeStrategy,
    return_entity_group: bool
) -> Result<ReconciliationOutcome, anyhow::Error> where C: ConnectionTrait {
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
    let locally_changed_entities = local_log.iter().map(|entry| (entry.target_type.clone().into(), entry.target_uuid)).collect::<HashSet<_>>();

    let mut remote_log_models = changes.log.iter().enumerate().map(
        |(idx, entry)| {
            open_tab_entities::schema::tournament_log::Model {
                uuid: entry.uuid,
                tournament_id,
                target_type: entry.target_type.as_str().clone().into(),
                target_uuid: entry.target_uuid,
                timestamp: entry.timestamp,
                sequence_idx: head_sequence_idx + idx as i32 + 1
            }.into_active_model()
        }
    ).collect_vec();

    // This unwrap is safe, since we reject an empty log with no last common ancestor
    let new_last_common_ancestor = remote_log_models.last().map(|model| model.uuid.clone().unwrap()).unwrap_or_else(|| last_common_ancestor.unwrap());
    let new_head_idx = remote_log_models.last().map(|model| model.sequence_idx.clone().unwrap()).unwrap_or(head_sequence_idx);

    let remote_changes_entities = changes.entities.iter().flat_map(|(entity_type, entries)| {
        entries.iter().map(|entry| (entity_type.clone(), entry.uuid))
    }).collect::<HashSet<_>>();

    let conflicting_entities = locally_changed_entities.intersection(&remote_changes_entities).collect::<HashSet<_>>();

    conflicting_entities.iter().enumerate().for_each(|(idx, (entity_type, uuid))| {
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
    
    // We bypass the normal save logic here, since we save the entire log at once
    let mut entities_to_save = vec![];
    for (entity_type, entities) in changes.entities.into_iter() {
        for entry in entities {
            if !conflicting_entities.contains(&(entity_type.clone(), entry.uuid)) {
                entities_to_save.push(entry.current_value);
            }
        }
    }

    let group = EntityGroup::from(entities_to_save);
    group.save_all(db).await?;    
    Ok(
        ReconciliationOutcome::Success {
            new_last_common_ancestor,
            entity_group: if return_entity_group { Some(group) } else { None }
        }
    )
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest<E, T> where T: EntityTypeId {
    pub log: FatLog<E, T>,
    pub last_common_ancestor: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequestResponse {
    pub outcome: APIReconciliationOutcome
}


async fn handle_sync_push_request(
    State(db): State<DatabaseConnection>,
    ExtractAuthenticatedUser(user): ExtractAuthenticatedUser,
    Path(tournament_id): Path<Uuid>,
    Json(request_body): Json<SyncRequest<Entity, EntityType>>
) -> Result<Json<SyncRequestResponse>, APIError> {
    if !user.check_is_authorized_for_tournament_administration(&db, tournament_id).await? {
        return Err(APIError::new("User is not authorized for tournament administration".into()));
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
        false
    ).await?;

    transaction.commit().await.map_err(handle_error)?;

    Ok(
        Json(
            SyncRequestResponse {
                outcome: outcome.into()
            }
        )
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
    .route("/tournament/:tournament_id/log", get(get_log))
    .route("/tournament/:tournament_id/log", post(handle_sync_push_request))
}