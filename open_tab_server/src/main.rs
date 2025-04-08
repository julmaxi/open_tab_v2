
use clap::{parser, Parser};
use open_tab_server::{commands::Command, config, db::DatabaseConfig, state::AppState};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}


#[tokio::main]
async fn main() {
    let parser = Cli::parse();
    let config = config::read_config();
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                config.logging_config.clone().into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = open_tab_server::db::set_up_db(
        DatabaseConfig::new(
            config.db_url.clone(),
        )
    ).await.expect("Failed to set up database");
    let app_state = AppState::new_with_db(db).await;

    match parser.command {
        Some(c) => {
            c.run(app_state).await.expect("Failed to run command");
            return;
        }
        None => {
            let app = open_tab_server::app_with_state(app_state).await;
            axum::Server::bind(&format!("{}:{}", config.host, config.port).parse().unwrap())
            .serve(app.into_make_service())
            .await
            .expect("Failed to start server");
        }
    }
}
