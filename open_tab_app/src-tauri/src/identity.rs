use std::collections::HashMap;


pub struct IdentityProvider {
    known_keys: tokio::sync::RwLock<HashMap<String, String>>,
    pending_identity_requests: tokio::sync::Mutex<HashMap<String, tokio::sync::broadcast::Sender<String>>>
}

impl IdentityProvider {
    pub fn new() -> Self {
        Self {
            known_keys: tokio::sync::RwLock::new(HashMap::new()),
            pending_identity_requests: tokio::sync::Mutex::new(HashMap::new())
        }
    }

    pub fn new_with_keys(keys: HashMap<String, String>) -> Self {
        Self {
            known_keys: tokio::sync::RwLock::new(keys),
            pending_identity_requests: tokio::sync::Mutex::new(HashMap::new())
        }
    }

    pub async fn get_key_blocking(&self, remote: &str) -> Result<String, anyhow::Error> {
        let known_keys = self.known_keys.read().await;
        let known_key = known_keys.get(remote).cloned();
        drop(known_keys);
        match known_key {
            Some(key) => {
                Ok(key.clone())
            }
            None => {
                let mut pending_identity_requests = self.pending_identity_requests.lock().await;
                let tx = pending_identity_requests.entry(remote.to_string()).or_insert_with(|| {
                    let (tx, _) = tokio::sync::broadcast::channel(1);
                    tx
                });

                let mut subscription = tx.subscribe();
                drop(pending_identity_requests);
                let key = subscription.recv().await?;

                Ok(key)
            }
        }
    }
    
    pub async fn try_get_key(&self, remote: &str) -> Option<String> {
        self.known_keys.read().await.get(remote).cloned()
    }

    pub async fn invalidate_key(&self, remote: &str) {
        self.known_keys.write().await.remove(remote);
    }

    pub async fn set_key(&self, remote: String, key: String) {
        self.known_keys.write().await.insert(remote.clone(), key.clone());
        let mut pending_identity_requests = self.pending_identity_requests.lock().await;
        if let Some(tx) = pending_identity_requests.remove(&remote) {
            tx.send(key).unwrap();
        }
    }
}
