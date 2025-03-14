use open_tab_entities::{info::TournamentParticipantsInfo, schema, tab::TabView};
use sea_orm::{prelude::Uuid, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, ColumnTrait};
use serde::Deserialize;
use tokio::sync::RwLock;

struct SerializedLRUCacheEntry {
    version: Uuid,
    data: Vec<u8>,
}

struct SerializedLRUCache<K> {
    cache: lru::LruCache<K, SerializedLRUCacheEntry>,
    max_size: usize,
    curr_size: usize
}

impl<K> SerializedLRUCache<K> where K: std::hash::Hash + Eq + Clone {
    fn new(max_size: usize) -> Self {
        Self {
            cache: lru::LruCache::unbounded(),
            max_size,
            curr_size: 0
        }
    }

    fn get<'a, V>(&'a mut self, key: &K) -> anyhow::Result<Option<(Uuid, V)>> where V: Deserialize<'a> {
        let data = self.cache.get(key);

        data.map(|data| (data.version, bincode::deserialize(&data.data))).map(|(v, r)| match r {
            Ok(r) => Ok((v, r)),
            Err(e) => Err(anyhow::Error::new(e))
        }).transpose()
    }

    fn insert<V>(&mut self, key: K, version: Uuid, value: &V) -> anyhow::Result<bool> where V: serde::Serialize {
        let data = bincode::serialize(value)?;

        let prev_entry = self.cache.pop(&key);
        if let Some(entry) = prev_entry {
            self.curr_size -= entry.data.len();
        }

        if data.len() > self.max_size {
            return Ok(false);
        }

        while data.len() > (self.max_size - self.curr_size) {
            let popped = self.cache.pop_lru();
            self.curr_size -= popped.unwrap().1.data.len();
        }
        
        self.curr_size += data.len();
        self.cache.put(key, SerializedLRUCacheEntry {
            version,
            data
        });

        Ok(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CacheKey {
    Tab(Uuid, Vec<Uuid>, bool),
    ParticipantInfo(Uuid),
}

pub struct CacheManager {
    cache: RwLock<SerializedLRUCache<CacheKey>>,
}

impl CacheManager {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(SerializedLRUCache::new(max_size))
        }
    }
    pub async fn get_tournament_participants_info<C>(&self, tournament_id: Uuid, db: &C) -> anyhow::Result<TournamentParticipantsInfo> where C: ConnectionTrait {
        let current_tournament_version = schema::tournament_log::Entity::find()
            .filter(schema::tournament_log::Column::TournamentId.eq(tournament_id))
            .order_by_desc(schema::tournament_log::Column::SequenceIdx)
            .one(db)
            .await?
            .map(|log| log.uuid);

        let current_tournament_version = current_tournament_version.unwrap_or_default();

        let key = CacheKey::ParticipantInfo(tournament_id);

        let mut cache = self.cache.write().await;

        match cache.get::<TournamentParticipantsInfo>(&key) {
            Ok(Some((version, info))) if version == current_tournament_version => {
                tracing::debug!("Cache hit for tournament participants info with key {:?}", key);
                Ok(info)
            }
            _ => {
                tracing::debug!("Cache miss for tournament participants info with key {:?}", key);
                let info = TournamentParticipantsInfo::load(db, tournament_id).await?;
                cache.insert(key, current_tournament_version, &info)?;
                Ok(info)
            }
        }
    }

    pub async fn get_tab<C>(&self, tournament_id: Uuid, round_ids: Vec<Uuid>, show_anonymity: bool, db: &C) -> anyhow::Result<TabView> where C: ConnectionTrait {
        let current_tournament_version = schema::tournament_log::Entity::find()
            .filter(schema::tournament_log::Column::TournamentId.eq(tournament_id))
            .order_by_desc(schema::tournament_log::Column::SequenceIdx)
            .one(db)
            .await?
            .map(|log| log.uuid);

        let current_tournament_version = current_tournament_version.unwrap_or_default();

        let key = CacheKey::Tab(tournament_id, round_ids.clone(), show_anonymity);

        let mut cache = self.cache.write().await;

        match cache.get::<TabView>(&key) {
            Ok(Some((version, tab))) if version == current_tournament_version => {
                tracing::debug!("Cache hit for tab with key {:?}", key);
                Ok(tab)
            }
            _ => {
                drop(cache);
                tracing::debug!("Cache miss for tab with key {:?}", key);
                let participant_info = self.get_tournament_participants_info(tournament_id, db).await?;
                let mut cache = self.cache.write().await;
                let tab = TabView::load_from_rounds_with_anonymity(db, round_ids, &participant_info, show_anonymity).await?;
                cache.insert(key, current_tournament_version, &tab)?;
                Ok(tab)
            }
        }
    }
}