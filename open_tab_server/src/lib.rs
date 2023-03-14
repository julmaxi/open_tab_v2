use serde::{Serialize, Deserialize};
use open_tab_entities::Entity;
use uuid::Uuid;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TournamentUpdate {
    pub changes: Vec<Entity>,
    pub expected_log_head: Option<Uuid>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentUpdateResponse {
    pub new_log_head: Uuid
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentChanges {
    pub changes: Vec<Entity>,
    pub log_head: Uuid
}