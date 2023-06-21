pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20230514_170224_add_users;
mod m20230618_115644_add_feedback;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230514_170224_add_users::Migration),
            Box::new(m20230618_115644_add_feedback::Migration),
        ]
    }
}
