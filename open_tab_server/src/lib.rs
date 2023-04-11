use std::error::Error;

use rocket::{response::status::Custom, http::Status};
use serde::{Serialize, Deserialize};
use open_tab_entities::Entity;
use uuid::Uuid;

pub mod ballots;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TournamentUpdate {
    pub changes: Vec<Entity>,
    pub expected_log_head: Option<Uuid>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentUpdateResponse {
    pub new_log_head: Uuid
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentChanges {
    pub changes: Vec<Entity>,
    pub log_head: Uuid
}

pub fn handle_error<E>(e: E) -> Custom<String> where E: Error {
    handle_error_impl(format!("{}", e))
}

pub fn handle_error_dyn<E>(e: Box<E>) -> Custom<String> where E: Error + ?Sized {
    handle_error_impl(format!("{}", e))
}

fn handle_error_impl(msg: String) -> Custom<String> {
    println!("{}", msg);
    Custom(Status::InternalServerError, msg)
}