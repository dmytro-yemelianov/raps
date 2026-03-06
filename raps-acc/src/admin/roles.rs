// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account role lookup for role name → role ID resolution

use anyhow::{Context, Result};

use raps_kernel::http;

use super::types::{AccountRole, Bim360Role};
use super::{AccountAdminClient, normalize_account_id};

impl AccountAdminClient {
    /// List all roles available in an account.
    ///
    /// Tries the ACC Construction Admin v1 endpoint first; falls back to
    /// BIM 360 HQ v2 on HTTP 400.
    pub async fn list_roles(&self, account_id: &str) -> Result<Vec<AccountRole>> {
        let account_id = normalize_account_id(account_id);

        match self.list_roles_acc(&account_id).await {
            Ok(roles) => Ok(roles),
            Err(e) if e.to_string().contains("400") || e.to_string().contains("404") => {
                self.list_roles_bim360(&account_id).await
            }
            Err(e) => Err(e),
        }
    }

    /// Resolve a role value (name or ID) to a role ID.
    ///
    /// - If the value is already a UUID, returns it unchanged.
    /// - Otherwise calls `list_roles` and matches by name (case-insensitive).
    /// - On no match, returns an error listing the available role names.
    pub async fn resolve_role_id(&self, account_id: &str, role: &str) -> Result<String> {
        if is_uuid(role) {
            return Ok(role.to_string());
        }

        let roles = self
            .list_roles(account_id)
            .await
            .context("Failed to list account roles for role name resolution")?;

        // Exact match first, then case-insensitive
        if let Some(r) = roles.iter().find(|r| r.name == role) {
            return Ok(r.id.clone());
        }
        if let Some(r) = roles
            .iter()
            .find(|r| r.name.to_lowercase() == role.to_lowercase())
        {
            return Ok(r.id.clone());
        }

        // Partial match
        if let Some(r) = roles
            .iter()
            .find(|r| r.name.to_lowercase().contains(&role.to_lowercase()))
        {
            return Ok(r.id.clone());
        }

        let available: Vec<String> = roles.iter().map(|r| format!("\"{}\"", r.name)).collect();
        anyhow::bail!(
            "Role {:?} not found. Available roles: {}",
            role,
            available.join(", ")
        )
    }

    async fn list_roles_acc(&self, account_id: &str) -> Result<Vec<AccountRole>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/roles", self.admin_url(account_id));

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list ACC roles (HTTP {status}): {body}");
        }

        // ACC v1 roles endpoint returns {"results": [...]} or plain array
        let body = response.text().await?;
        parse_roles_response(&body)
    }

    async fn list_roles_bim360(&self, account_id: &str) -> Result<Vec<AccountRole>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/roles", self.hq_v2_url(account_id));

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list BIM 360 roles (HTTP {status}): {body}");
        }

        // BIM 360 v2 roles returns plain array with snake_case fields
        let roles: Vec<Bim360Role> = response
            .json()
            .await
            .context("Failed to parse BIM 360 roles response")?;

        Ok(roles
            .into_iter()
            .map(|r| AccountRole { id: r.id, name: r.name })
            .collect())
    }
}

/// Parse roles from either `{"results":[...]}` or a plain `[...]` array
fn parse_roles_response(body: &str) -> Result<Vec<AccountRole>> {
    // Try plain array first
    if let Ok(roles) = serde_json::from_str::<Vec<AccountRole>>(body) {
        return Ok(roles);
    }
    // Try {"results": [...]} wrapper
    #[derive(serde::Deserialize)]
    struct Wrapped {
        results: Vec<AccountRole>,
    }
    let wrapped: Wrapped = serde_json::from_str(body)
        .context("Failed to parse roles response (expected array or {results:[]})")?;
    Ok(wrapped.results)
}

/// Returns true if the string looks like a UUID (with or without dashes)
fn is_uuid(s: &str) -> bool {
    let s = s.replace('-', "");
    s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())
}
