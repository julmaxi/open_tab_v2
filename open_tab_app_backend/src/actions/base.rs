

use async_trait::async_trait;
use open_tab_entities::{prelude::*};

use sea_orm::prelude::*;


#[async_trait]
pub trait ActionTrait {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, anyhow::Error> where C: ConnectionTrait;
}
