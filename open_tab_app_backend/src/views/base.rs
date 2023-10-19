
use std::{collections::HashMap, error::Error};

use migration::async_trait::async_trait;
use open_tab_entities::domain::tournament_institution::TournamentInstitution;


use sea_orm::prelude::*;
use open_tab_entities::prelude::*;




use itertools::Itertools;

pub use open_tab_entities::info::TournamentParticipantsInfo;

#[async_trait]
pub trait LoadedView : Sync + Send {
    // We can't use a connection trait here, since otherwise the trait is not object safe
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, Box<dyn Error>>;
    async fn view_string(&self) -> Result<String, Box<dyn Error>>;
}
