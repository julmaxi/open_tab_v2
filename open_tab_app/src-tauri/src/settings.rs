use std::{collections::HashMap, fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{App, AppHandle, Manager, State};
use tokio::sync::RwLock;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub known_remotes: Vec<RemoteSettings>,
    pub known_api_keys: HashMap<String, String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSettings {
    pub url: String,
    pub name: String,
    pub account_name: Option<String>,
}

impl AppSettings {
    pub fn settings_path() -> PathBuf {
        let settings_dir = dirs::config_dir().unwrap_or(PathBuf::from(".")).join("com.juliussteen.open-tab");
        let settings_path = settings_dir.join("settings.json");
        settings_path
    }

    pub fn try_read() -> Result<Self, anyhow::Error> {
        let settings_path = Self::settings_path();
        let settings_file = File::open(&settings_path)?;

        let settings = serde_json::from_reader(settings_file)?;
        Ok(settings)
    }

    pub fn write(&self) -> Result<(), anyhow::Error> {
        let path = Self::settings_path();
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        let settings_file = File::create(&path)?;

        serde_json::to_writer(settings_file, &self)?;
        Ok(())
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            known_remotes: vec![
                RemoteSettings {
                    url: "https://api.debateresult.com".to_string(),
                    name: "Default".to_string(),
                    account_name: None,
                },
                RemoteSettings {
                    url: "http://localhost:3000".to_string(),
                    name: "Local".to_string(),
                    account_name: None,
                }
            ],
            known_api_keys: HashMap::new()
        }
    }
}


#[tauri::command]
pub async fn get_settings(settings: State<'_, RwLock<AppSettings>>) -> Result<AppSettings, ()> {
    Ok(settings.inner().read().await.clone())
}


#[tauri::command]
pub async fn add_remote(app: AppHandle, settings: State<'_, RwLock<AppSettings>>, new_remote: RemoteSettings) -> Result<(), ()> {
    let mut settings = settings.write().await;

    settings.known_remotes.retain(|r| r.url != new_remote.url);
    settings.known_remotes.push(new_remote);
    update_settings(app, &mut settings).map_err(|_| ())?;

    Ok(())
}

#[tauri::command]
pub async fn remove_remote(app: AppHandle, settings: State<'_, RwLock<AppSettings>>, url: String) -> Result<(), ()> {
    let mut settings = settings.write().await;

    settings.known_remotes.retain(|r| r.url != url);
    update_settings(app, &mut settings).map_err(|_| ())?;

    Ok(())
}

fn update_settings(app: AppHandle, settings: &mut AppSettings) -> Result<(), anyhow::Error> {
    app.emit_all("settings-changed", settings.clone())?;
    settings.write()?;

    Ok(())
}

/*
#[tauri::command]
pub async fn change_settings(settings: State<'_, RwLock<AppSettings>>, new_settings: AppSettings) -> Result<(), ()> {
    new_settings.write().map_err(|_| ())?;

    let mut settings = settings.write().await;
    *settings = new_settings;
    Ok(())
}
*/