// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP middleware for request/response processing
//!
//! Provides middleware components for:
//! - Request logging
//! - Response logging
//! - Header injection
//! - Secret redaction

use crate::logging::redact_secrets;
use tracing::{debug, trace};

/// Request logging middleware
///
/// Logs HTTP requests at debug level with method, URL, and headers.
pub fn log_request(method: &str, url: &str, headers: Option<&[(&str, &str)]>) {
    let redacted_url = redact_secrets(url);
    debug!(method = %method, url = %redacted_url, "HTTP request");

    if let Some(hdrs) = headers {
        for (name, value) in hdrs {
            // Redact authorization headers
            let display_value = if name.to_lowercase() == "authorization" {
                "[REDACTED]"
            } else {
                value
            };
            trace!(header = %name, value = %display_value, "Request header");
        }
    }
}

/// Response logging middleware
///
/// Logs HTTP responses at debug level with status and URL.
pub fn log_response(status: u16, url: &str, duration_ms: u64) {
    let redacted_url = redact_secrets(url);
    debug!(
        status = %status,
        url = %redacted_url,
        duration_ms = %duration_ms,
        "HTTP response"
    );
}

/// Standard APS request headers
pub struct ApsHeaders;

impl ApsHeaders {
    /// User-Agent header value
    pub const USER_AGENT: &'static str = concat!("raps/", env!("CARGO_PKG_VERSION"));

    /// Content-Type for JSON
    pub const CONTENT_TYPE_JSON: &'static str = "application/json";

    /// Content-Type for form-urlencoded
    pub const CONTENT_TYPE_FORM: &'static str = "application/x-www-form-urlencoded";

    /// Accept header for JSON
    pub const ACCEPT_JSON: &'static str = "application/json";

    /// Get standard headers for APS API requests
    pub fn standard() -> Vec<(&'static str, &'static str)> {
        vec![
            ("User-Agent", Self::USER_AGENT),
            ("Accept", Self::ACCEPT_JSON),
        ]
    }

    /// Get headers for JSON body requests
    pub fn json() -> Vec<(&'static str, &'static str)> {
        vec![
            ("User-Agent", Self::USER_AGENT),
            ("Accept", Self::ACCEPT_JSON),
            ("Content-Type", Self::CONTENT_TYPE_JSON),
        ]
    }

    /// Get headers for form data requests
    pub fn form() -> Vec<(&'static str, &'static str)> {
        vec![
            ("User-Agent", Self::USER_AGENT),
            ("Accept", Self::ACCEPT_JSON),
            ("Content-Type", Self::CONTENT_TYPE_FORM),
        ]
    }
}

/// Request modifier trait for middleware chain
pub trait RequestModifier {
    /// Modify a request before sending
    fn modify_request(&self, headers: &mut Vec<(String, String)>);
}

/// Bearer token auth modifier
pub struct BearerAuth {
    token: String,
}

impl BearerAuth {
    /// Create new bearer auth modifier
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

impl RequestModifier for BearerAuth {
    fn modify_request(&self, headers: &mut Vec<(String, String)>) {
        headers.push((
            "Authorization".to_string(),
            format!("Bearer {}", self.token),
        ));
    }
}

/// Region header modifier for APS multi-region support
pub struct RegionHeader {
    region: String,
}

impl RegionHeader {
    /// Create new region header modifier
    ///
    /// Valid regions: "US", "EMEA"
    pub fn new(region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
        }
    }
}

impl RequestModifier for RegionHeader {
    fn modify_request(&self, headers: &mut Vec<(String, String)>) {
        headers.push(("x-ads-region".to_string(), self.region.clone()));
    }
}

/// Request ID header for tracing
pub struct RequestId {
    id: String,
}

impl RequestId {
    /// Create new request ID modifier with auto-generated UUID
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create with specific ID
    pub fn with_id(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// Get the request ID
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestModifier for RequestId {
    fn modify_request(&self, headers: &mut Vec<(String, String)>) {
        headers.push(("x-request-id".to_string(), self.id.clone()));
    }
}

/// Apply multiple modifiers to headers
pub fn apply_modifiers(modifiers: &[&dyn RequestModifier], headers: &mut Vec<(String, String)>) {
    for modifier in modifiers {
        modifier.modify_request(headers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aps_headers_standard() {
        let headers = ApsHeaders::standard();
        assert_eq!(headers.len(), 2);
        assert!(headers.iter().any(|(k, _)| *k == "User-Agent"));
        assert!(headers.iter().any(|(k, _)| *k == "Accept"));
    }

    #[test]
    fn test_aps_headers_json() {
        let headers = ApsHeaders::json();
        assert_eq!(headers.len(), 3);
        assert!(headers
            .iter()
            .any(|(k, v)| *k == "Content-Type" && *v == "application/json"));
    }

    #[test]
    fn test_aps_headers_form() {
        let headers = ApsHeaders::form();
        assert!(headers
            .iter()
            .any(|(k, v)| *k == "Content-Type" && *v == "application/x-www-form-urlencoded"));
    }

    #[test]
    fn test_bearer_auth_modifier() {
        let auth = BearerAuth::new("test_token");
        let mut headers = Vec::new();
        auth.modify_request(&mut headers);

        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "Authorization");
        assert_eq!(headers[0].1, "Bearer test_token");
    }

    #[test]
    fn test_region_header_modifier() {
        let region = RegionHeader::new("EMEA");
        let mut headers = Vec::new();
        region.modify_request(&mut headers);

        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "x-ads-region");
        assert_eq!(headers[0].1, "EMEA");
    }

    #[test]
    fn test_request_id_modifier() {
        let request_id = RequestId::new();
        let mut headers = Vec::new();
        request_id.modify_request(&mut headers);

        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "x-request-id");
        // Should be a valid UUID
        assert!(!headers[0].1.is_empty());
    }

    #[test]
    fn test_request_id_custom() {
        let request_id = RequestId::with_id("custom-123");
        assert_eq!(request_id.id(), "custom-123");
    }

    #[test]
    fn test_apply_modifiers() {
        let auth = BearerAuth::new("token");
        let region = RegionHeader::new("US");
        let request_id = RequestId::with_id("req-123");

        let modifiers: Vec<&dyn RequestModifier> = vec![&auth, &region, &request_id];
        let mut headers = Vec::new();
        apply_modifiers(&modifiers, &mut headers);

        assert_eq!(headers.len(), 3);
        assert!(headers.iter().any(|(k, _)| k == "Authorization"));
        assert!(headers.iter().any(|(k, _)| k == "x-ads-region"));
        assert!(headers.iter().any(|(k, _)| k == "x-request-id"));
    }
}
