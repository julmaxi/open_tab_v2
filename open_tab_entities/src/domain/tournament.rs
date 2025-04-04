use async_trait::async_trait;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, SimpleEntity)]
#[module_path = "crate::schema::tournament"]
#[tournament_id = "uuid"]
#[skip_field = "last_modified"]
pub struct Tournament {
    pub uuid: Uuid,
    pub annoucements_password: Option<String>,
    pub name: String,
    pub feedback_release_time: Option<DateTime>,
    pub allow_self_declared_clashes: bool,
    pub allow_speaker_self_declared_clashes: bool,
    pub show_declared_clashes: bool,
}


impl Tournament {
    pub fn new() -> Self {
        Tournament {
            uuid: Uuid::new_v4(),
            annoucements_password: None,
            name: "New Tournament".into(),
            feedback_release_time: None,
            allow_self_declared_clashes: false,
            allow_speaker_self_declared_clashes: false,
            show_declared_clashes: false,
        }
    }
}
