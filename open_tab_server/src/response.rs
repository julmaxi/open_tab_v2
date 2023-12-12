use std::{str::FromStr};

use axum::response::{IntoResponse, Response};


use serde::{Serialize, Deserialize};
use tracing::error;

pub type APIError = TypedAPIError<String>;

#[derive(Debug, Clone)]
pub struct TypedAPIError<T> {
    pub message: T,
    pub code: axum::http::StatusCode
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIErrorResponse<T> {
    pub message: T
}

impl APIError {
    pub fn new(message: String) -> Self {
        APIError {
            message,
            code: axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

impl<T> From<anyhow::Error> for TypedAPIError<T> where T: From<String> {
    fn from(err: anyhow::Error) -> Self {
        error!("Error while handling request {}", err.to_string());
        TypedAPIError { message: err.to_string().into(), code: axum::http::StatusCode::INTERNAL_SERVER_ERROR }
    }
}

impl<T> From<(axum::http::StatusCode, T)> for TypedAPIError<T> where T: Serialize + std::fmt::Debug {
    fn from((code, err): (axum::http::StatusCode, T)) -> Self {
        error!("Error while handling request {:?}", err);
        TypedAPIError { message: err, code }
    }
}

impl<T> IntoResponse for TypedAPIError<T> where T: Serialize
{
    fn into_response(self) -> Response {
        let mut res = serde_json::to_string(&APIErrorResponse {message: self.message}).unwrap().into_response();
        *res.status_mut() = self.code;
        res
    }
}

impl From<(axum::http::StatusCode, &str)> for APIError {
    fn from((code, message): (axum::http::StatusCode, &str)) -> Self {
        error!("Error while handling request {}", message);
        APIError { message: message.to_string(), code }
    }
}

/*
impl From<(axum::http::StatusCode, String)> for APIError {
    fn from((code, message): (axum::http::StatusCode, String)) -> Self {
        error!("Error while handling request {}", message);
        APIError { message: message.to_string(), code }
    }
}
 */

impl FromStr for APIError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(APIError { message: s.to_string(), code: axum::http::StatusCode::INTERNAL_SERVER_ERROR })
    }
}

pub fn handle_error<E>(err: E) -> APIError
where
    E: std::error::Error
{
    error!("Error while handling request {}", err);
    APIError::new(err.to_string())
}


pub fn handle_typed_error<E, T>(err: E) -> TypedAPIError<T>
where
    E: std::error::Error,
    T: From<String>
{
    error!("Error while handling request {}", err);
    TypedAPIError { message: err.to_string().into(), code: axum::http::StatusCode::INTERNAL_SERVER_ERROR }
}


pub fn handle_error_dyn(err: Box<dyn std::error::Error>) -> APIError
{
    error!("Error while handling request {}", err);
    APIError::new(err.to_string())
}