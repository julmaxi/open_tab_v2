
use open_tab_server::{state::AppState, db::DatabaseConfig};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, serde::Deserialize)]
#[serde(default)]
struct Config {
    db_name: String,
    db_url: String,
    host: String,
    port: u16,
    logging_config: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            db_name: "".to_string(),
            db_url: "sqlite://./server.sqlite3?mode=rwc".into(),
            host: "0.0.0.0".into(),
            port: 3000,
            logging_config: "trace,sqlx::query=debug,hyper=error,mio=debug,tower_http=debug,axum::rejection=trace,sqlx::query=error".into()
        }
    }
}

fn read_config() -> Config {
    let local_path = std::path::Path::new("./config.yml");

    if local_path.exists() {
        let config = std::fs::read_to_string(local_path).unwrap();
        let config = serde_yaml::from_str::<Config>(&config);
        if let Err(e) = config {
            println!("Failed to parse config file: {}", e);
            Config::default()
        }
        else {
            config.unwrap()
        }
    } else {
        println!("No config file found, using defaults");
        Config::default()
    }
}

#[tokio::main]
async fn main() {
    let config = read_config();
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                config.logging_config.clone().into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = open_tab_server::db::set_up_db(
        DatabaseConfig::new(
            config.db_url.clone(),
            config.db_name.clone(),
        )
    ).await.expect("Failed to set up database");

    let app_state = AppState::new_with_db(db).await;
    // build our application with a route
    let app = open_tab_server::app_with_state(app_state).await;

    // run our app with hyper, listening globally on port 3000
    axum::Server::bind(&format!("{}:{}", config.host, config.port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
