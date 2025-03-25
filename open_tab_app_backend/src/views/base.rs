
use std::{any::Any, collections::HashMap};

use async_trait::async_trait;



use sea_orm::prelude::*;
use open_tab_entities::prelude::*;






pub use open_tab_entities::info::TournamentParticipantsInfo;


pub trait LoadedViewToAny: 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> LoadedViewToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}


#[async_trait]
pub trait LoadedView : Sync + Send + LoadedViewToAny {
    // We can't use a connection trait here, since otherwise the trait is not object safe
    async fn update_and_get_changes(&mut self, db: &sea_orm::DatabaseTransaction, changes: &EntityGroup) -> Result<Option<HashMap<String, serde_json::Value>>, anyhow::Error>;
    async fn view_string(&self) -> Result<String, anyhow::Error>;
}


