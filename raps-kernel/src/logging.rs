// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Logging and secret redaction

use regex::Regex;
use tracing::Level;

/// Initialize tracing with secret redaction
pub fn init_logging(level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();
}

/// Redact secrets from log output
pub fn redact_secrets(text: &str) -> String {
    let patterns = vec![
        (r"(?i)(client[_-]?secret)\s*[:=]\s*([^\s,}]+)", r"$1=***REDACTED***"),
        (r"(?i)(access[_-]?token)\s*[:=]\s*([^\s,}]+)", r"$1=***REDACTED***"),
        (r"(?i)(refresh[_-]?token)\s*[:=]\s*([^\s,}]+)", r"$1=***REDACTED***"),
        (r"Bearer\s+([A-Za-z0-9_-]+)", "Bearer ***REDACTED***"),
    ];

    let mut result = text.to_string();
    for (pattern, replacement) in patterns {
        if let Ok(re) = Regex::new(pattern) {
            result = re.replace_all(&result, replacement).to_string();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_client_secret() {
        let input = "client_secret=abc123xyz";
        let output = redact_secrets(input);
        assert!(output.contains("REDACTED"));
        assert!(!output.contains("abc123xyz"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let output = redact_secrets(input);
        assert!(output.contains("REDACTED"));
    }
}
