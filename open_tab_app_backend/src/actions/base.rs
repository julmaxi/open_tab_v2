use std::{error::Error};

use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*};

use sea_orm::prelude::*;


#[async_trait]
pub trait ActionTrait {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroup, Box<dyn Error>> where C: ConnectionTrait;
}
