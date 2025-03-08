use std::{fmt::Error, str::FromStr};

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

impl<T> TypedAPIError<T> {
    pub fn new_with_status<S>(code: axum::http::StatusCode, message: S) -> Self where S: Into<T> {
        TypedAPIError {
            message: message.into(),
            code
        }
    }
}

/*
impl<T> From<anyhow::Error> for TypedAPIError<T> where T: From<String> {
    fn from(err: anyhow::Error) -> Self {
        error!("Error while handling request {}", err.to_string());
        TypedAPIError { message: err.to_string().into(), code: axum::http::StatusCode::INTERNAL_SERVER_ERROR }
    }
}
 */

impl<E> From<E> for APIError where E: std::fmt::Display {
    fn from(err: E) -> Self {
        APIError { message: err.to_string(), code: axum::http::StatusCode::INTERNAL_SERVER_ERROR }
    }
}

impl<T> IntoResponse for TypedAPIError<T> where T: Serialize + std::fmt::Display
{
    fn into_response(self) -> Response {
        tracing::error!("Error while handling request: {}", self.message);
        let mut res = serde_json::to_string(&APIErrorResponse {
            message: self.message
        }).unwrap().into_response();
        *res.status_mut() = self.code;
        res
    }
}
