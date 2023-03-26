use std::{error::Error, fmt::{Display, Formatter}, collections::HashMap};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*};

use sea_orm::prelude::*;

use crate::draw_view::DrawBallot;
use serde::{Serialize, Deserialize};


#[async_trait]
pub trait ActionTrait {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroups, Box<dyn Error>> where C: ConnectionTrait;
}