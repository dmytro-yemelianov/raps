// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    RateLimited { retry_after: u64 },
    Internal(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::RateLimited { retry_after } => {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [("Retry-After", retry_after.to_string())],
                    Json(json!({"error": "rate_limited", "message": "Too many requests", "retry_after": retry_after})),
                )
                    .into_response();
            }
            ApiError::Internal(err) => {
                tracing::error!("Internal error: {err:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        (
            status,
            Json(json!({"error": status.as_str(), "message": message})),
        )
            .into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
            sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
                ApiError::Conflict("Resource already exists".to_string())
            }
            _ => ApiError::Internal(err.into()),
        }
    }
}
