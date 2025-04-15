use std::any;

use axum::{body::Body, extract::State, http::{Request, StatusCode}, middleware::Next, response::Response, Router};
use clap::builder::Str;
use open_tab_entities::schema::asset;
use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, ActiveValue, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};
use serde::{Deserialize, Serialize};

use crate::{db, state::AppState};
use tower_http::services::ServeDir;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetFileType {
    #[serde(rename = "png")]
    Png,
    #[serde(rename = "jpeg")]
    Jpeg,
    #[serde(rename = "svg")]
    Svg,
}

impl AssetFileType {
    pub fn get_mime_type(&self) -> &'static str {
        match self {
            AssetFileType::Png => "image/png",
            AssetFileType::Jpeg => "image/jpeg",
            AssetFileType::Svg => "image/svg+xml"
        }
    }

    pub fn get_extension(&self) -> &'static str {
        match self {
            AssetFileType::Png => "png",
            AssetFileType::Jpeg => "jpeg",
            AssetFileType::Svg => "svg"
        }
    }

    pub fn from_filename(s: &str) -> anyhow::Result<Self> {
        let parts = s.rsplit_once(".");
        let out = parts.map(|(_, ext)| {
            let ext_lc = ext.to_lowercase();
            match ext_lc.as_str() {
                "png" => Ok(AssetFileType::Png),
                "jpeg" => Ok(AssetFileType::Jpeg),
                "svg" => Ok(AssetFileType::Svg),
                _ => Err(anyhow::anyhow!("Unknown file type: {}", ext_lc))
            }
        }).transpose()?;
        match out {
            Some(v) => Ok(v),
            None => Err(anyhow::anyhow!("No extension found"))
        }
    }
}

pub async fn save_named_asset(
    state: &AppState,
    content: Vec<u8>,
    file_type: AssetFileType,
    name: String,
) -> anyhow::Result<Uuid> {
    let db = state.db.clone();
    let asset_id = Uuid::new_v4();
    let assets_path = state.config.assets_path.clone();
    let asset_path = format!("{}/{}", assets_path, asset_id);

    let transaction = db.begin().await?;
    let hash = seahash::hash(&content);

    let asset = open_tab_entities::schema::asset::ActiveModel {
        uuid: ActiveValue::Set(asset_id),
        hash: ActiveValue::Set(hash.to_be_bytes().to_vec()),
        name: ActiveValue::Set(Some(name)),
        file_type: ActiveValue::Set(serde_json::to_string(&file_type)?),
    };

    asset.insert(&transaction).await?;

    std::fs::write(&asset_path, content).expect("Failed to write asset file");
    transaction
        .commit()
        .await?;

    Ok(asset_id)
}



async fn asset_middleware<B>(
    State(app_state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // do something with `request`...

    let path = request.uri().path().rsplit_once("/");
    if let Some((_, path)) = path {
        let uuid = Uuid::parse_str(path).map_err(|_| StatusCode::BAD_REQUEST)?;

        let asset = asset::Entity::find()
            .filter(asset::Column::Uuid.eq(uuid.clone()))
            .one(&app_state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        let file_type: AssetFileType = serde_json::from_str(&asset.file_type)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mime_type = file_type.get_mime_type();
        let mut response = next.run(request).await;
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            mime_type.parse().unwrap(),
        );
        response.headers_mut().insert(
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}.{}\"", asset.name.unwrap_or_else(|| uuid.to_string()), file_type.get_extension()).parse().unwrap(),
        );
        return Ok(response);
    }
    else {
        return Err(StatusCode::NOT_FOUND);
    }
}

pub(crate) fn router(app_state: &AppState) -> Router<AppState> {
    Router::new()
        .nest_service("/assets", ServeDir::new(app_state.config.assets_path.clone()).append_index_html_on_directories(false))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            asset_middleware
        ))
        
}
