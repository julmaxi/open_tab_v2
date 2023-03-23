pub mod draw_view;
pub mod tab_view;
mod base;

pub use self::base::LoadedView;

use std::error::Error;

use self::draw_view::LoadedDrawView;

use sea_orm::ConnectionTrait;
use sea_orm::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum View {
    Draw{uuid: Uuid}
}

impl View {
    pub async fn load_json<C>(&self, db: &C) -> Result<String, Box<dyn Error>> where C: ConnectionTrait {
        /*match self {
            View::Draw{uuid} => {
                let draw_view = DrawView::load(db, *uuid).await?;
                Ok(serde_json::to_string(&draw_view)?)
            }
        }*/
        let view = self.load(db).await?;
        view.view_string().await
    }

    pub async fn load<C>(&self, db: &C) -> Result<Box<dyn LoadedView>, Box<dyn Error>> where C: ConnectionTrait {
        Ok(match self {
            View::Draw{uuid} => {
                Box::new(LoadedDrawView::load(db, *uuid).await?)
            }
        })
    }
}