// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin API client for ACC/BIM 360

mod companies;
mod projects;
mod roles;
mod types;
mod users;

#[cfg(test)]
mod tests;

pub use types::*;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Client for ACC Account Admin API
///
/// Provides operations for managing users and projects at the account level.
/// Requires account admin privileges.
pub struct AccountAdminClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl AccountAdminClient {
    /// Create a new Account Admin client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create client with custom HTTP configuration
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> Self {
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            auth,
            http_client,
        }
    }

    /// Get the base URL for Account Admin API
    fn admin_url(&self, account_id: &str) -> String {
        format!(
            "{}/construction/admin/v1/accounts/{}",
            self.config.base_url, account_id
        )
    }

    /// Get the base URL for HQ v1 API (used for companies endpoint)
    fn hq_url(&self, account_id: &str) -> String {
        format!("{}/hq/v1/accounts/{}", self.config.base_url, account_id)
    }

    /// Get the base URL for BIM 360 HQ v2 API
    pub(crate) fn hq_v2_url(&self, account_id: &str) -> String {
        format!("{}/hq/v2/accounts/{}", self.config.base_url, account_id)
    }
}

/// Normalize account ID to the format expected by ACC Admin API
///
/// Handles various input formats:
/// - `b.{uuid}` (BIM 360 hub format) -> extracts uuid
/// - `a.{base64}` (ACC hub format) -> decodes and extracts account ID
/// - Raw UUID -> returns as-is
pub(crate) fn normalize_account_id(account_id: &str) -> String {
    // Handle BIM 360 format: b.{uuid}
    if let Some(id) = account_id.strip_prefix("b.") {
        return id.to_string();
    }

    // Handle ACC format: a.{base64}
    if let Some(encoded) = account_id.strip_prefix("a.")
        && let Ok(decoded_bytes) = base64_decode(encoded)
        && let Ok(decoded) = String::from_utf8(decoded_bytes)
    {
        // Format is typically "business:{account_id}" or just the account_id
        if let Some(id) = decoded.strip_prefix("business:") {
            return id.to_string();
        }
        // Try splitting on colon for other formats
        if let Some((_, id)) = decoded.split_once(':') {
            return id.to_string();
        }
        return decoded;
    }

    // Already a raw account ID
    account_id.to_string()
}

/// Simple base64 decoder (URL-safe variant)
fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    STANDARD.decode(input).map_err(|_| ())
}
