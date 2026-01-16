// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Error types for bulk admin operations

use uuid::Uuid;

/// Errors from bulk operations
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("User not found in account: {email}")]
    UserNotFound { email: String },

    #[error("Invalid filter expression: {message}")]
    InvalidFilter { message: String },

    #[error("Operation not found: {id}")]
    OperationNotFound { id: Uuid },

    #[error("Operation cannot be resumed (status: {status})")]
    CannotResume { status: String },

    #[error("Invalid operation: {message}")]
    InvalidOperation { message: String },

    #[error("Rate limit exceeded, retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("State persistence error: {0}")]
    StateError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Standard exit codes for bulk operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// All items processed successfully
    Success = 0,
    /// Some items failed
    PartialSuccess = 1,
    /// Operation could not start
    Failure = 2,
    /// User cancelled
    Cancelled = 3,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        code as i32
    }
}
