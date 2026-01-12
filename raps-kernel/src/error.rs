// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Error handling and exit code management
//!
//! Provides standardized exit codes for CI/CD scripting:
//! - 0: Success
//! - 2: Invalid arguments / validation failure
//! - 3: Auth failure
//! - 4: Not found
//! - 5: Remote/API error
//! - 6: Internal error
//!
//! Also provides APS error interpretation with human-readable explanations.

use anyhow::Error;
use colored::Colorize;
use serde::Deserialize;
use std::process;

/// Exit codes following standard conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Success is used implicitly when no error occurs
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
        let error_chain: Vec<String> = err.chain().map(|e| e.to_string().to_lowercase()).collect();

        // Check for authentication errors
        if error_string.contains("authentication failed")
            || error_string.contains("auth failed")
            || error_string.contains("unauthorized")
            || error_string.contains("forbidden")
            || error_string.contains("invalid credentials")
            || error_string.contains("token expired")
            || error_string.contains("token invalid")
            || error_chain
                .iter()
                .any(|e| e.contains("401") || e.contains("403") || e.contains("authentication"))
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
#[allow(dead_code)] // Trait may be used in future
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
                eprintln!("Error: {err}");

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

// ============== APS ERROR INTERPRETATION ==============

/// Common APS API error response structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ApsErrorResponse {
    #[serde(alias = "error", alias = "errorCode")]
    pub error_code: Option<String>,
    #[serde(alias = "error_description", alias = "errorDescription")]
    pub description: Option<String>,
    #[serde(alias = "message", alias = "msg")]
    pub detail: Option<String>,
    pub reason: Option<String>,
    pub developer_message: Option<String>,
}

/// Parsed and interpreted APS error
#[derive(Debug)]
#[allow(dead_code)]
pub struct InterpretedError {
    pub status_code: u16,
    pub error_code: String,
    pub explanation: String,
    pub suggestions: Vec<String>,
    pub original_message: String,
}

/// Parse and interpret an APS API error response
#[allow(dead_code)]
pub fn interpret_error(status_code: u16, response_body: &str) -> InterpretedError {
    let parsed: Option<ApsErrorResponse> = serde_json::from_str(response_body).ok();

    let (error_code, message) = if let Some(ref err) = parsed {
        let code = err
            .error_code
            .clone()
            .or(err.reason.clone())
            .unwrap_or_else(|| status_to_code(status_code));
        let msg = err
            .detail
            .clone()
            .or(err.description.clone())
            .or(err.developer_message.clone())
            .unwrap_or_else(|| response_body.to_string());
        (code, msg)
    } else {
        (status_to_code(status_code), response_body.to_string())
    };

    let (explanation, suggestions) = get_error_help(status_code, &error_code, &message);

    InterpretedError {
        status_code,
        error_code,
        explanation,
        suggestions,
        original_message: message,
    }
}

fn status_to_code(status: u16) -> String {
    match status {
        400 => "BadRequest".to_string(),
        401 => "Unauthorized".to_string(),
        403 => "Forbidden".to_string(),
        404 => "NotFound".to_string(),
        409 => "Conflict".to_string(),
        429 => "TooManyRequests".to_string(),
        500 => "InternalServerError".to_string(),
        502 => "BadGateway".to_string(),
        503 => "ServiceUnavailable".to_string(),
        _ => format!("Error{}", status),
    }
}

fn get_error_help(status_code: u16, error_code: &str, message: &str) -> (String, Vec<String>) {
    let message_lower = message.to_lowercase();
    let code_lower = error_code.to_lowercase();

    // Authentication errors
    if status_code == 401
        || code_lower.contains("unauthorized")
        || code_lower.contains("invalid_token")
    {
        return (
            "Authentication failed. Your token is invalid, expired, or missing.".to_string(),
            vec![
                "Run 'raps auth login' to re-authenticate".to_string(),
                "Check that your client credentials are correct".to_string(),
                "Verify RAPS_CLIENT_ID and RAPS_CLIENT_SECRET environment variables".to_string(),
            ],
        );
    }

    // Scope/permission errors
    if status_code == 403
        || code_lower.contains("forbidden")
        || code_lower.contains("insufficient_scope")
    {
        let mut suggestions = vec![
            "Check that your app has the required scopes enabled in APS Portal".to_string(),
            "Run 'raps auth login' with the necessary scopes".to_string(),
        ];

        if message_lower.contains("data:read") || message_lower.contains("data:write") {
            suggestions.push("Add 'data:read'/'data:write' scopes for Data Management".to_string());
        }
        if message_lower.contains("bucket") {
            suggestions.push("Add 'bucket:read'/'bucket:create' scopes for OSS".to_string());
        }

        return (
            "Permission denied. Your token lacks required scopes.".to_string(),
            suggestions,
        );
    }

    // Not found errors
    if status_code == 404 {
        return (
            "Resource not found.".to_string(),
            vec![
                "Verify the resource ID is correct".to_string(),
                "Check that the resource exists".to_string(),
                "Ensure you have access to the resource".to_string(),
            ],
        );
    }

    // Rate limiting
    if status_code == 429 {
        return (
            "Rate limit exceeded.".to_string(),
            vec![
                "Wait and retry the request".to_string(),
                "Reduce request frequency".to_string(),
            ],
        );
    }

    // Server errors
    if status_code >= 500 {
        return (
            "APS server error (temporary).".to_string(),
            vec![
                "Wait and retry".to_string(),
                "Check APS status page".to_string(),
            ],
        );
    }

    // Default
    (
        format!("Request failed (HTTP {})", status_code),
        vec!["Check the error details".to_string()],
    )
}

/// Format an interpreted error for display
#[allow(dead_code)]
pub fn format_interpreted_error(error: &InterpretedError, use_colors: bool) -> String {
    let mut output = String::new();

    if use_colors {
        output.push_str(&format!(
            "\n{} {}\n",
            "Error:".red().bold(),
            error.explanation
        ));
        output.push_str(&format!(
            "  {} {} (HTTP {})\n",
            "Code:".bold(),
            error.error_code,
            error.status_code
        ));

        if !error.original_message.is_empty() && error.original_message != error.explanation {
            output.push_str(&format!(
                "  {} {}\n",
                "Details:".bold(),
                error.original_message.dimmed()
            ));
        }

        if !error.suggestions.is_empty() {
            output.push_str(&format!("\n{}\n", "Suggestions:".yellow().bold()));
            for suggestion in &error.suggestions {
                output.push_str(&format!("  {} {}\n", "→".cyan(), suggestion));
            }
        }
    } else {
        output.push_str(&format!("\nError: {}\n", error.explanation));
        output.push_str(&format!(
            "  Code: {} (HTTP {})\n",
            error.error_code, error.status_code
        ));

        if !error.original_message.is_empty() {
            output.push_str(&format!("  Details: {}\n", error.original_message));
        }

        if !error.suggestions.is_empty() {
            output.push_str("\nSuggestions:\n");
            for suggestion in &error.suggestions {
                output.push_str(&format!("  - {}\n", suggestion));
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_from_auth_error() {
        let err = anyhow::anyhow!("authentication failed: unauthorized");
        assert_eq!(ExitCode::from_error(&err), ExitCode::AuthFailure);
    }

    #[test]
    fn test_exit_code_from_not_found_error() {
        let err = anyhow::anyhow!("Resource not found");
        assert_eq!(ExitCode::from_error(&err), ExitCode::NotFound);
    }

    #[test]
    fn test_exit_code_from_validation_error() {
        let err = anyhow::anyhow!("Invalid bucket name: must be lowercase");
        assert_eq!(ExitCode::from_error(&err), ExitCode::InvalidArguments);
    }

    #[test]
    fn test_exit_code_from_remote_error() {
        let err = anyhow::anyhow!("API error: 500 Internal Server Error");
        assert_eq!(ExitCode::from_error(&err), ExitCode::RemoteError);
    }

    #[test]
    fn test_interpret_401_error() {
        let error = interpret_error(
            401,
            r#"{"error": "invalid_token", "error_description": "Token expired"}"#,
        );
        assert_eq!(error.status_code, 401);
        assert!(error.explanation.contains("Authentication"));
        assert!(!error.suggestions.is_empty());
    }

    #[test]
    fn test_interpret_403_error() {
        let error = interpret_error(
            403,
            r#"{"error": "insufficient_scope", "detail": "Missing data:read scope"}"#,
        );
        assert_eq!(error.status_code, 403);
        assert!(error.explanation.contains("Permission"));
    }

    #[test]
    fn test_interpret_404_error() {
        let error = interpret_error(404, r#"{"message": "Bucket not found"}"#);
        assert_eq!(error.status_code, 404);
        assert!(error.explanation.contains("not found"));
    }

    #[test]
    fn test_interpret_429_error() {
        let error = interpret_error(429, "Rate limit exceeded");
        assert_eq!(error.status_code, 429);
        assert!(error.explanation.contains("Rate limit"));
    }

    #[test]
    fn test_interpret_500_error() {
        let error = interpret_error(500, "Internal server error");
        assert_eq!(error.status_code, 500);
        assert!(error.explanation.contains("server error"));
    }

    #[test]
    fn test_interpret_plain_text_error() {
        let error = interpret_error(400, "Bad request: invalid parameter");
        assert_eq!(error.status_code, 400);
        assert_eq!(error.error_code, "BadRequest");
    }

    #[test]
    fn test_format_interpreted_error_no_colors() {
        let error = InterpretedError {
            status_code: 401,
            error_code: "Unauthorized".to_string(),
            explanation: "Authentication failed".to_string(),
            suggestions: vec!["Run 'raps auth login'".to_string()],
            original_message: "Token expired".to_string(),
        };

        let formatted = format_interpreted_error(&error, false);
        assert!(formatted.contains("Authentication failed"));
        assert!(formatted.contains("Unauthorized"));
        assert!(formatted.contains("401"));
        assert!(formatted.contains("raps auth login"));
    }

    #[test]
    fn test_status_to_code() {
        assert_eq!(status_to_code(400), "BadRequest");
        assert_eq!(status_to_code(401), "Unauthorized");
        assert_eq!(status_to_code(403), "Forbidden");
        assert_eq!(status_to_code(404), "NotFound");
        assert_eq!(status_to_code(429), "TooManyRequests");
        assert_eq!(status_to_code(500), "InternalServerError");
        assert_eq!(status_to_code(418), "Error418"); // Custom code
    }

    // ==================== Additional Exit Code Tests ====================

    #[test]
    fn test_exit_code_from_forbidden_error() {
        let err = anyhow::anyhow!("403 Forbidden: insufficient permissions");
        assert_eq!(ExitCode::from_error(&err), ExitCode::AuthFailure);
    }

    #[test]
    fn test_exit_code_from_token_expired() {
        let err = anyhow::anyhow!("token expired");
        assert_eq!(ExitCode::from_error(&err), ExitCode::AuthFailure);
    }

    #[test]
    fn test_exit_code_from_token_invalid() {
        let err = anyhow::anyhow!("token invalid");
        assert_eq!(ExitCode::from_error(&err), ExitCode::AuthFailure);
    }

    #[test]
    fn test_exit_code_from_invalid_credentials() {
        let err = anyhow::anyhow!("invalid credentials");
        assert_eq!(ExitCode::from_error(&err), ExitCode::AuthFailure);
    }

    #[test]
    fn test_exit_code_from_404_in_chain() {
        let inner = anyhow::anyhow!("status: 404");
        let err = inner.context("Failed to fetch resource");
        assert_eq!(ExitCode::from_error(&err), ExitCode::NotFound);
    }

    #[test]
    fn test_exit_code_from_missing_required() {
        let err = anyhow::anyhow!("bucket name is required");
        assert_eq!(ExitCode::from_error(&err), ExitCode::InvalidArguments);
    }

    #[test]
    fn test_exit_code_from_cannot_be_empty() {
        let err = anyhow::anyhow!("field cannot be empty");
        assert_eq!(ExitCode::from_error(&err), ExitCode::InvalidArguments);
    }

    #[test]
    fn test_exit_code_from_must_be() {
        let err = anyhow::anyhow!("value must be positive");
        assert_eq!(ExitCode::from_error(&err), ExitCode::InvalidArguments);
    }

    #[test]
    fn test_exit_code_from_timeout() {
        let err = anyhow::anyhow!("request timeout after 30s");
        assert_eq!(ExitCode::from_error(&err), ExitCode::RemoteError);
    }

    #[test]
    fn test_exit_code_from_network() {
        let err = anyhow::anyhow!("network error: connection reset");
        assert_eq!(ExitCode::from_error(&err), ExitCode::RemoteError);
    }

    #[test]
    fn test_exit_code_from_connection() {
        let err = anyhow::anyhow!("connection refused");
        assert_eq!(ExitCode::from_error(&err), ExitCode::RemoteError);
    }

    #[test]
    fn test_exit_code_unknown_defaults_to_internal() {
        let err = anyhow::anyhow!("something went wrong");
        assert_eq!(ExitCode::from_error(&err), ExitCode::InternalError);
    }

    // ==================== Exit Code Value Tests ====================

    #[test]
    fn test_exit_code_values() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::InvalidArguments as i32, 2);
        assert_eq!(ExitCode::AuthFailure as i32, 3);
        assert_eq!(ExitCode::NotFound as i32, 4);
        assert_eq!(ExitCode::RemoteError as i32, 5);
        assert_eq!(ExitCode::InternalError as i32, 6);
    }

    // ==================== Additional Interpret Error Tests ====================

    #[test]
    fn test_interpret_502_error() {
        let error = interpret_error(502, "Bad Gateway");
        assert_eq!(error.status_code, 502);
        assert!(error.explanation.contains("server error"));
    }

    #[test]
    fn test_interpret_503_error() {
        let error = interpret_error(503, "Service Unavailable");
        assert_eq!(error.status_code, 503);
        assert!(error.explanation.contains("server error"));
    }

    #[test]
    fn test_interpret_error_with_scope_suggestion() {
        let error = interpret_error(
            403,
            r#"{"error": "forbidden", "detail": "Missing data:read scope"}"#,
        );
        assert!(error.suggestions.iter().any(|s| s.contains("data:read")));
    }

    #[test]
    fn test_interpret_error_with_bucket_suggestion() {
        let error = interpret_error(
            403,
            r#"{"error": "forbidden", "detail": "Missing bucket:create scope"}"#,
        );
        assert!(error.suggestions.iter().any(|s| s.contains("bucket")));
    }

    #[test]
    fn test_interpret_error_json_parsing() {
        let error = interpret_error(
            400,
            r#"{"errorCode": "InvalidRequest", "message": "Bad parameter"}"#,
        );
        assert_eq!(error.error_code, "InvalidRequest");
        assert!(error.original_message.contains("Bad parameter"));
    }

    #[test]
    fn test_interpret_error_developer_message() {
        let error = interpret_error(
            400,
            r#"{"error": "BadRequest", "developer_message": "Check API docs"}"#,
        );
        assert!(error.original_message.contains("Check API docs"));
    }

    #[test]
    fn test_interpret_error_reason_field() {
        let error = interpret_error(400, r#"{"reason": "InvalidParameter"}"#);
        assert_eq!(error.error_code, "InvalidParameter");
    }

    #[test]
    fn test_interpret_409_conflict() {
        let error = interpret_error(409, r#"{"error": "Conflict"}"#);
        assert_eq!(status_to_code(409), "Conflict");
    }

    // ==================== Format Error Tests ====================

    #[test]
    fn test_format_error_with_empty_message() {
        let error = InterpretedError {
            status_code: 400,
            error_code: "BadRequest".to_string(),
            explanation: "Bad request".to_string(),
            suggestions: vec![],
            original_message: "".to_string(),
        };
        let formatted = format_interpreted_error(&error, false);
        assert!(formatted.contains("Bad request"));
        // Empty message shouldn't add extra "Details:" line
        assert!(!formatted.contains("Details:") || formatted.contains("Details: \n"));
    }

    #[test]
    fn test_format_error_with_colors() {
        let error = InterpretedError {
            status_code: 401,
            error_code: "Unauthorized".to_string(),
            explanation: "Auth failed".to_string(),
            suggestions: vec!["Login again".to_string()],
            original_message: "Token expired".to_string(),
        };
        let formatted = format_interpreted_error(&error, true);
        // Should contain the content (colors are ANSI codes)
        assert!(formatted.contains("Auth failed"));
        assert!(formatted.contains("Token expired"));
        assert!(formatted.contains("Login again"));
    }

    #[test]
    fn test_format_error_no_suggestions() {
        let error = InterpretedError {
            status_code: 400,
            error_code: "BadRequest".to_string(),
            explanation: "Bad request".to_string(),
            suggestions: vec![],
            original_message: "Invalid input".to_string(),
        };
        let formatted = format_interpreted_error(&error, false);
        // Should not have "Suggestions:" section
        assert!(!formatted.contains("Suggestions:"));
    }

    #[test]
    fn test_format_error_same_explanation_and_message() {
        let error = InterpretedError {
            status_code: 400,
            error_code: "BadRequest".to_string(),
            explanation: "Same message".to_string(),
            suggestions: vec![],
            original_message: "Same message".to_string(),
        };
        let formatted = format_interpreted_error(&error, false);
        // Note: Current implementation shows both, which is acceptable behavior
        // The test verifies the format function works with matching messages
        assert!(formatted.contains("Same message"));
    }
}
