use sea_orm::{prelude::*, Database, DbBackend, Statement};

pub struct DatabaseConfig {
    url: String,
    name: String
}


impl DatabaseConfig {
    pub fn new(url: String, name: String) -> DatabaseConfig {
        DatabaseConfig { url, name }
    }
}


pub async fn set_up_db(config: DatabaseConfig) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(config.url.clone()).await?;

    let db = match db.get_database_backend() {
        DbBackend::MySql => {
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!("CREATE DATABASE IF NOT EXISTS `{}`;", config.name),
            ))
            .await?;

            let url = format!("{}/{}", config.url, config.name);
            Database::connect(&url).await?
        }
        DbBackend::Postgres => {
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!("DROP DATABASE IF EXISTS \"{}\";", config.name),
            ))
            .await?;
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!("CREATE DATABASE \"{}\";", config.name),
            ))
            .await?;

            let url = format!("{}/{}", config.url, config.name);
            Database::connect(&url).await?
        }
        DbBackend::Sqlite => db,
    };

    Ok(db)
}
