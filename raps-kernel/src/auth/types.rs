// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication types for APS OAuth 2.0

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::types::StoredToken;

/// User profile information from /userinfo endpoint
#[derive(Debug, Clone, Deserialize)]
// API response structs may contain fields we don't use - this is expected for external API contracts
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
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Cached token with expiry tracking (for 2-legged)
#[derive(Debug, Clone)]
pub(crate) struct CachedToken {
    pub(crate) access_token: String,
    pub(crate) expires_at: Instant,
}

impl CachedToken {
    pub(crate) fn is_valid(&self) -> bool {
        self.expires_at > Instant::now() + Duration::from_secs(60)
    }
}

/// Cached 3-legged token with refresh coordination
#[derive(Debug, Clone)]
pub(crate) struct TokenCache {
    pub(crate) token: Option<StoredToken>,
    pub(crate) refreshing: bool,
}
