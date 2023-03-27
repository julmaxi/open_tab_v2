use std::{error::Error, fmt::{Display, Formatter}, collections::HashMap};

use itertools::{Itertools, izip};
use migration::async_trait::async_trait;
use open_tab_entities::{prelude::*};

use sea_orm::prelude::*;

use crate::{draw_view::DrawBallot, participants_list_view::ParticipantEntry, import::CSVReaderConfig};
use serde::{Serialize, Deserialize};

use super::ActionTrait;

//use crate::import::;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadParticipantsListAction {
    path: String,
    tournament_id: Uuid,
    parser_config: CSVReaderConfig
}



/*
#[async_trait]
impl ActionTrait for UploadParticipantsListAction {
    async fn get_changes<C>(self, db: &C) -> Result<EntityGroups, Box<dyn Error>> where C: ConnectionTrait {
    }
} */