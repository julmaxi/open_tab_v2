use sea_orm_migration::prelude::*;
pub use sea_orm_migration::prelude::{MigrationTrait, MigratorTrait};

mod m20220101_000001_create_table;
mod m20230514_170224_add_users;
mod m20230618_115644_add_feedback;
mod m20231125_213519_add_adj_break;
mod m20231128_191922_add_ballot_backup_index;
mod m20231129_174220_add_feedback_release;
mod m20231129_233804_add_confidential_feedback_questions;
mod m20231130_175355_add_participant_privacy_switch;
mod m20231210_154339_conform_to_rk_whims;
mod m20231228_094035_add_key_expiry;
mod m20240102_170321_add_speech_timing;
mod m20240128_123739_update_speech_fk;
mod m20240224_191827_add_speech_pause;
mod m20240611_200708_add_publication_settings;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20230514_170224_add_users::Migration),
            Box::new(m20230618_115644_add_feedback::Migration),
            Box::new(m20231125_213519_add_adj_break::Migration),
            Box::new(m20231128_191922_add_ballot_backup_index::Migration),
            Box::new(m20231129_174220_add_feedback_release::Migration),
            Box::new(m20231129_233804_add_confidential_feedback_questions::Migration),
            Box::new(m20231130_175355_add_participant_privacy_switch::Migration),
            Box::new(m20231210_154339_conform_to_rk_whims::Migration),
            Box::new(m20231228_094035_add_key_expiry::Migration),
            Box::new(m20240102_170321_add_speech_timing::Migration),
            Box::new(m20240128_123739_update_speech_fk::Migration),
            Box::new(m20240224_191827_add_speech_pause::Migration),
            Box::new(m20240611_200708_add_publication_settings::Migration),
        ]
    }
}
