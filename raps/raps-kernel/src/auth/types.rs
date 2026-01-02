// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication types for APS OAuth 2.0

use serde::{Deserialize, Serialize};

/// User profile information from /userinfo endpoint
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct UserInfo {
    /// The unique APS ID of the user
    pub sub: String,
    /// Full name
    pub name: Option<String>,
    /// First name
    pub given_name: Option<String>,
    /// Last name
    pub family_name: Option<String>,
    /// Preferred username
    pub preferred_username: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Whether email is verified
    pub email_verified: Option<bool>,
    /// Profile URL
    pub profile: Option<String>,
    /// Profile picture URL
    pub picture: Option<String>,
}

/// OAuth 2.0 token response from APS
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    /// Access token string
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiration time in seconds
    pub expires_in: u64,
    /// Refresh token (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// OAuth scopes granted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Device code response from APS Device Authorization endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    /// Device code for polling
    pub device_code: String,
    /// User code to display
    pub user_code: String,
    /// Verification URI
    pub verification_uri: String,
    /// Complete verification URI with user code
    pub verification_uri_complete: Option<String>,
    /// Expiration time in seconds
    pub expires_in: u64,
    /// Polling interval in seconds
    pub interval: Option<u64>,
}

/// Stored token with metadata for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    /// Access token string
    pub access_token: String,
    /// Refresh token (if available)
    pub refresh_token: Option<String>,
    /// Expiration timestamp (Unix epoch seconds)
    pub expires_at: i64,
    /// OAuth scopes granted
    pub scopes: Vec<String>,
}

impl StoredToken {
    /// Check if token is still valid (not expired)
    ///
    /// Returns false if token will expire within 60 seconds.
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        // Consider expired 60 seconds before actual expiry
        self.expires_at > now + 60
    }

    /// Create a new stored token from a token response
    pub fn from_response(response: &TokenResponse) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            access_token: response.access_token.clone(),
            refresh_token: response.refresh_token.clone(),
            expires_at: now + response.expires_in as i64,
            scopes: response
                .scope
                .as_ref()
                .map(|s| s.split_whitespace().map(String::from).collect())
                .unwrap_or_default(),
        }
    }

    /// Time remaining until token expires (in seconds)
    pub fn time_remaining(&self) -> i64 {
        let now = chrono::Utc::now().timestamp();
        self.expires_at - now
    }
}

/// OAuth 2.0 scopes for APS APIs
pub struct Scopes;

impl Scopes {
    /// Read data from OSS, derivative, and data management
    pub const DATA_READ: &'static str = "data:read";
    /// Write data to OSS, derivative, and data management
    pub const DATA_WRITE: &'static str = "data:write";
    /// Create new data in OSS and data management
    pub const DATA_CREATE: &'static str = "data:create";
    /// Search data
    pub const DATA_SEARCH: &'static str = "data:search";

    /// Read bucket information
    pub const BUCKET_READ: &'static str = "bucket:read";
    /// Create buckets
    pub const BUCKET_CREATE: &'static str = "bucket:create";
    /// Update bucket policies
    pub const BUCKET_UPDATE: &'static str = "bucket:update";
    /// Delete buckets
    pub const BUCKET_DELETE: &'static str = "bucket:delete";

    /// Full code access (Design Automation)
    pub const CODE_ALL: &'static str = "code:all";

    /// Read user profile
    pub const USER_READ: &'static str = "user:read";
    /// Read user profile (alternate)
    pub const USER_PROFILE_READ: &'static str = "user-profile:read";

    /// Read account information
    pub const ACCOUNT_READ: &'static str = "account:read";
    /// Write account information
    pub const ACCOUNT_WRITE: &'static str = "account:write";

    /// All viewable data
    pub const VIEWABLES_READ: &'static str = "viewables:read";

    /// Default scopes for 2-legged authentication
    pub fn two_legged_default() -> Vec<&'static str> {
        vec![
            Self::DATA_READ,
            Self::DATA_WRITE,
            Self::DATA_CREATE,
            Self::BUCKET_READ,
            Self::BUCKET_CREATE,
            Self::BUCKET_DELETE,
            Self::CODE_ALL,
        ]
    }

    /// Default scopes for 3-legged authentication
    pub fn three_legged_default() -> Vec<&'static str> {
        vec![
            Self::DATA_READ,
            Self::DATA_WRITE,
            Self::DATA_CREATE,
            Self::USER_READ,
            Self::USER_PROFILE_READ,
            Self::VIEWABLES_READ,
        ]
    }

    /// Join scopes with space separator
    pub fn join(scopes: &[&str]) -> String {
        scopes.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stored_token_validity() {
        let now = chrono::Utc::now().timestamp();

        // Valid token (expires in 1 hour)
        let token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now + 3600,
            scopes: vec!["data:read".to_string()],
        };
        assert!(token.is_valid());

        // Expired token
        let expired_token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now - 100,
            scopes: vec!["data:read".to_string()],
        };
        assert!(!expired_token.is_valid());

        // Token expiring soon (within 60 seconds) should be invalid
        let soon_expiring = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now + 30,
            scopes: vec!["data:read".to_string()],
        };
        assert!(!soon_expiring.is_valid());
    }

    #[test]
    fn test_stored_token_from_response() {
        let response = TokenResponse {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("refresh".to_string()),
            scope: Some("data:read data:write".to_string()),
        };

        let stored = StoredToken::from_response(&response);
        assert_eq!(stored.access_token, "test_token");
        assert_eq!(stored.refresh_token, Some("refresh".to_string()));
        assert!(stored.scopes.contains(&"data:read".to_string()));
        assert!(stored.scopes.contains(&"data:write".to_string()));
    }

    #[test]
    fn test_stored_token_time_remaining() {
        let now = chrono::Utc::now().timestamp();
        let token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: now + 3600,
            scopes: vec![],
        };

        let remaining = token.time_remaining();
        // Should be approximately 3600 seconds (allow 2 second margin)
        assert!(remaining >= 3598 && remaining <= 3600);
    }

    #[test]
    fn test_token_response_serialization() {
        let token = TokenResponse {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("refresh_token".to_string()),
            scope: None,
        };

        let json = serde_json::to_string(&token).expect("serialize");
        assert!(json.contains("test_token"));
        assert!(json.contains("Bearer"));
        assert!(json.contains("refresh_token"));

        let deserialized: TokenResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.access_token, "test_token");
        assert_eq!(deserialized.token_type, "Bearer");
        assert_eq!(deserialized.expires_in, 3600);
        assert_eq!(
            deserialized.refresh_token,
            Some("refresh_token".to_string())
        );
    }

    #[test]
    fn test_scopes_join() {
        let scopes = Scopes::join(&[Scopes::DATA_READ, Scopes::DATA_WRITE]);
        assert_eq!(scopes, "data:read data:write");
    }

    #[test]
    fn test_two_legged_default_scopes() {
        let scopes = Scopes::two_legged_default();
        assert!(scopes.contains(&"data:read"));
        assert!(scopes.contains(&"bucket:create"));
        assert!(scopes.contains(&"code:all"));
    }

    #[test]
    fn test_three_legged_default_scopes() {
        let scopes = Scopes::three_legged_default();
        assert!(scopes.contains(&"data:read"));
        assert!(scopes.contains(&"user:read"));
        assert!(scopes.contains(&"viewables:read"));
    }
}
