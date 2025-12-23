//! Error handling and exit code management
//!
//! Provides standardized exit codes for CI/CD scripting:
//! - 0: Success
//! - 2: Invalid arguments / validation failure
//! - 3: Auth failure
//! - 4: Not found
//! - 5: Remote/API error
//! - 6: Internal error

use anyhow::Error;
use std::process;

/// Exit codes following standard conventions
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
    /// Determine exit code from an error
    ///
    /// Analyzes the error chain to determine the appropriate exit code
    pub fn from_error(err: &Error) -> Self {
        let error_string = err.to_string().to_lowercase();
        let error_chain: Vec<String> = err
            .chain()
            .map(|e| e.to_string().to_lowercase())
            .collect();

        // Check for authentication errors
        if error_string.contains("authentication failed")
            || error_string.contains("auth failed")
            || error_string.contains("unauthorized")
            || error_string.contains("forbidden")
            || error_string.contains("invalid credentials")
            || error_string.contains("token expired")
            || error_string.contains("token invalid")
            || error_chain.iter().any(|e| {
                e.contains("401") || e.contains("403") || e.contains("authentication")
            })
        {
            return ExitCode::AuthFailure;
        }

        // Check for not found errors
        if error_string.contains("not found")
            || error_string.contains("404")
            || error_chain.iter().any(|e| e.contains("404"))
        {
            return ExitCode::NotFound;
        }

        // Check for validation/argument errors
        if error_string.contains("invalid")
            || error_string.contains("validation")
            || error_string.contains("required")
            || error_string.contains("missing")
            || error_string.contains("cannot be empty")
            || error_string.contains("must be")
        {
            return ExitCode::InvalidArguments;
        }

        // Check for remote/API errors (5xx, network errors, etc.)
        if error_string.contains("500")
            || error_string.contains("502")
            || error_string.contains("503")
            || error_string.contains("504")
            || error_string.contains("timeout")
            || error_string.contains("connection")
            || error_string.contains("network")
            || error_string.contains("api")
            || error_string.contains("remote")
            || error_string.contains("server error")
            || error_chain.iter().any(|e| {
                e.contains("500")
                    || e.contains("502")
                    || e.contains("503")
                    || e.contains("504")
                    || e.contains("timeout")
                    || e.contains("connection")
            })
        {
            return ExitCode::RemoteError;
        }

        // Default to internal error for unknown errors
        ExitCode::InternalError
    }

    /// Exit the process with this exit code
    pub fn exit(self) -> ! {
        process::exit(self as i32);
    }
}

/// Extension trait for Result to easily exit with appropriate code
pub trait ResultExt<T> {
    /// Unwrap or exit with appropriate exit code
    fn unwrap_or_exit(self) -> T;
}

impl<T> ResultExt<T> for Result<T, Error> {
    fn unwrap_or_exit(self) -> T {
        match self {
            Ok(val) => val,
            Err(err) => {
                let exit_code = ExitCode::from_error(&err);
                eprintln!("Error: {}", err);
                
                // Print chain of errors
                let mut source = err.source();
                while let Some(cause) = source {
                    eprintln!("  Caused by: {}", cause);
                    source = cause.source();
                }
                
                exit_code.exit();
            }
        }
    }
}

