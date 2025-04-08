#[derive(Debug, serde::Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub db_url: String,
    pub host: String,
    pub port: u16,
    pub logging_config: String,
    #[serde(default = "assets_default_path")]
    pub assets_path: String
}

fn assets_default_path() -> String {
    "assets".into()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            db_url: "sqlite://./server.sqlite3?mode=rwc".into(),
            host: "0.0.0.0".into(),
            port: 3000,
            logging_config: "trace,sqlx::query=debug,hyper=error,mio=debug,tower_http=debug,axum::rejection=trace,sqlx::query=error".into(),
            assets_path: assets_default_path(),
        }
    }
}

pub(crate) fn read_config_inner() -> Result<Config, anyhow::Error> {
    let config_path = std::env::var("OPEN_TAB_SERVER_CONFIG")?;
    let config = std::fs::read_to_string(config_path)?;
    let config = serde_yaml::from_str::<Config>(&config)?;
    Ok(config)
}

pub fn read_config() -> Config {
    match read_config_inner() {
        Ok(config) => config,
        Err(e) => {
            //Print to stderr, since logging is set up in the config
            eprintln!("Warning: Failed to read config: {}", e);
            Config::default()
        }
    }
}
