use std::{str::FromStr, error::Error};

use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use serde::{Serialize, Deserialize};
use tracing::error;


#[derive(Debug, Clone)]
pub struct APIError {
    pub message: String,
    pub code: StatusCode
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct APIErrorResponse {
    message: String
}

impl APIError {
    pub fn new(message: String) -> Self {
        APIError {
            message,
            code: StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

impl IntoResponse for APIError
{
    fn into_response(self) -> Response {
        let mut res = serde_json::to_string(&APIErrorResponse {message: self.message.clone()}).unwrap().into_response();
        *res.status_mut() = self.code;
        res
    }
}


impl From<(StatusCode, &str)> for APIError {
    fn from((code, message): (StatusCode, &str)) -> Self {
        error!("Error while handling request {}", message);
        APIError { message: message.to_string(), code }
    }
}

impl From<(StatusCode, String)> for APIError {
    fn from((code, message): (StatusCode, String)) -> Self {
        error!("Error while handling request {}", message);
        APIError { message: message.to_string(), code }
    }
}

impl From<Box<dyn Error>> for APIError {
    fn from(err: Box<dyn Error>) -> Self {
        error!("Error while handling request {}", err.to_string());
        APIError { message: err.to_string(), code: StatusCode::INTERNAL_SERVER_ERROR }
    }
}

impl FromStr for APIError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(APIError { message: s.to_string(), code: StatusCode::INTERNAL_SERVER_ERROR })
    }
}

pub fn handle_error<E>(err: E) -> APIError
where
    E: std::error::Error
{
    error!("Error while handling request {}", err);
    APIError::new(err.to_string())
}



pub fn handle_error_dyn(err: Box<dyn std::error::Error>) -> APIError
{
    error!("Error while handling request {}", err);
    APIError::new(err.to_string())
}