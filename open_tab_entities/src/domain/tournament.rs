use async_trait::async_trait;
use open_tab_macros::SimpleEntity;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};


#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, SimpleEntity)]
#[module_path = "crate::schema::tournament"]
#[tournament_id = "uuid"]
pub struct Tournament {
    pub uuid: Uuid,
    pub annoucements_password: Option<String>,
    pub name: String,
    pub feedback_release_time: Option<DateTime>,
}


impl Tournament {
    pub fn new() -> Self {
        Tournament {
            uuid: Uuid::new_v4(),
            annoucements_password: None,
            name: "New Tournament".into(),
            feedback_release_time: None,
        }
    }
}
