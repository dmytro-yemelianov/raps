// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Error handling and exit code management for RAPS Kernel
//!
//! Provides standardized exit codes for CI/CD scripting:
//! - 0: Success
//! - 2: Invalid arguments / validation failure
//! - 3: Auth failure
//! - 4: Not found
//! - 5: Remote/API error
//! - 6: Internal error

use std::process;
use thiserror::Error;

/// Standard exit codes following conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Success
    Success = 0,
    /// Invalid arguments / validation failure
    InvalidArguments = 2,
    /// Authentication failure
    AuthFailure = 3,
    /// Resource not found
    NotFound = 4,
    /// Remote/API error
    RemoteError = 5,
    /// Internal error
    InternalError = 6,
}

impl ExitCode {
    /// Exit the process with this exit code
    pub fn exit(self) -> ! {
        process::exit(self as i32);
    }
}

/// RAPS Kernel error type
#[derive(Debug, Error)]
pub enum RapsError {
    /// Authentication error
    #[error("Authentication failed: {message}")]
    Auth {
        /// Error message
        message: String,
        /// Exit code for this error
        code: ExitCode,
        /// Underlying error source
        #[source]
        source: Option<anyhow::Error>,
    },

    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound {
        /// Resource identifier that was not found
        resource: String,
    },

    /// API/Network error
    #[error("API error: {message}")]
    Api {
        /// Error message
        message: String,
        /// HTTP status code (if available)
        status: Option<u16>,
        /// Underlying error source
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config {
        /// Error message
        message: String,
    },
    /// Storage error (token storage, file I/O, etc.)
    #[error("Storage error: {message}")]
    Storage {
        /// Error message
        message: String,
        /// Underlying error source
        #[source]
        source: Option<anyhow::Error>,
    },

    /// Network error
    #[error("Network error: {message}")]
    Network {
        /// Error message
        message: String,
        /// Underlying reqwest error
        #[source]
        source: Option<reqwest::Error>,
    },

    /// Internal error
    #[error("Internal error: {message}")]
    Internal {
        /// Error message
        message: String,
    },
}

impl RapsError {
    /// Get exit code for this error type
    pub fn exit_code(&self) -> ExitCode {
        match self {
            RapsError::Auth { code, .. } => *code,
            RapsError::NotFound { .. } => ExitCode::NotFound,
            RapsError::Api { .. } => ExitCode::RemoteError,
            RapsError::Config { .. } => ExitCode::InvalidArguments,
            RapsError::Network { .. } => ExitCode::RemoteError,
            RapsError::Storage { .. } => ExitCode::InternalError,
            RapsError::Internal { .. } => ExitCode::InternalError,
        }
    }
}

/// Result type alias for kernel operations
pub type Result<T> = std::result::Result<T, RapsError>;

impl From<reqwest::Error> for RapsError {
    fn from(err: reqwest::Error) -> Self {
        RapsError::Network {
            message: err.to_string(),
            source: Some(err),
        }
    }
}

impl From<serde_json::Error> for RapsError {
    fn from(err: serde_json::Error) -> Self {
        RapsError::Internal {
            message: format!("JSON parsing error: {}", err),
        }
    }
}

impl From<std::io::Error> for RapsError {
    fn from(err: std::io::Error) -> Self {
        RapsError::Internal {
            message: format!("I/O error: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_exit_code() {
        let err = RapsError::Auth {
            message: "Token expired".to_string(),
            code: ExitCode::AuthFailure,
            source: None,
        };
        assert_eq!(err.exit_code(), ExitCode::AuthFailure);
    }

    #[test]
    fn test_not_found_error_exit_code() {
        let err = RapsError::NotFound {
            resource: "bucket".to_string(),
        };
        assert_eq!(err.exit_code(), ExitCode::NotFound);
    }

    #[test]
    fn test_api_error_exit_code() {
        let err = RapsError::Api {
            message: "Server error".to_string(),
            status: Some(500),
            source: None,
        };
        assert_eq!(err.exit_code(), ExitCode::RemoteError);
    }

    #[test]
    fn test_config_error_exit_code() {
        let err = RapsError::Config {
            message: "Missing client_id".to_string(),
        };
        assert_eq!(err.exit_code(), ExitCode::InvalidArguments);
    }
}
