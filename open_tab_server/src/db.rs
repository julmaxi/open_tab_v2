use sea_orm::{prelude::*, Database, DbBackend, Statement};

pub struct DatabaseConfig {
    url: String,
}


impl DatabaseConfig {
    pub fn new(url: String) -> DatabaseConfig {
        DatabaseConfig { url}
    }
}


pub async fn set_up_db(config: DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(config.url.clone()).await?;
    Ok(db)
}
