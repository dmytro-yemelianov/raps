// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account role lookup for role name → role ID resolution

use anyhow::{Context, Result};

use raps_kernel::http;

use super::types::{AccountRole, Bim360Role, ResolvedRole};
use super::{AccountAdminClient, normalize_account_id};
use crate::types::ProductAccess;

impl AccountAdminClient {
    /// List all roles available in an account.
    ///
    /// Tries the ACC Construction Admin v1 endpoint first; falls back to
    /// BIM 360 HQ v2 project-level industry_roles on HTTP 400/404.
    /// For BIM 360 hubs, fetches from the first active project if no
    /// project_id is given (industry roles are typically shared across projects).
    pub async fn list_roles(&self, account_id: &str) -> Result<Vec<AccountRole>> {
        self.list_roles_with_project(account_id, None).await
    }

    /// List roles, optionally scoped to a specific project for BIM 360.
    pub async fn list_roles_with_project(
        &self,
        account_id: &str,
        project_id: Option<&str>,
    ) -> Result<Vec<AccountRole>> {
        let account_id = normalize_account_id(account_id);

        match self.list_roles_acc(&account_id).await {
            Ok(roles) => Ok(roles),
            Err(e) if e.to_string().contains("400") || e.to_string().contains("404") => {
                // BIM 360: roles are per-project (industry_roles endpoint)
                let pid = if let Some(p) = project_id {
                    p.to_string()
                } else {
                    // Fetch just the first page (1 API call) to find an active project
                    let page = self.list_projects(&account_id, Some(100), Some(0)).await?;
                    page.results
                        .into_iter()
                        .find(|p| p.status.as_deref() == Some("active"))
                        .map(|p| p.id)
                        .ok_or_else(|| anyhow::anyhow!(
                            "No active projects found in account to fetch BIM 360 industry roles. \
                             Use --project to specify a project ID."
                        ))?
                };
                self.list_roles_bim360(&account_id, &pid).await
            }
            Err(e) => Err(e),
        }
    }

    /// Resolve a role name to its representation for this hub type.
    ///
    /// - UUID input → `ResolvedRole::Uuid` (passed through, works for both hub types)
    /// - BIM 360 hub (roles endpoint returns data) → `ResolvedRole::Uuid` from name lookup
    /// - ACC hub (roles endpoint returns 404/empty) → `ResolvedRole::Products` from known mapping
    pub async fn resolve_role(&self, account_id: &str, role: &str) -> Result<ResolvedRole> {
        self.resolve_role_with_project(account_id, role, None).await
    }

    /// Resolve a role name, using a project_id hint to avoid extra API calls
    /// when fetching BIM 360 industry roles.
    pub async fn resolve_role_with_project(
        &self,
        account_id: &str,
        role: &str,
        project_id: Option<&str>,
    ) -> Result<ResolvedRole> {
        if is_uuid(role) {
            return Ok(ResolvedRole::Uuid(role.to_string()));
        }

        match self.list_roles_with_project(account_id, project_id).await {
            Ok(roles) if !roles.is_empty() => {
                // BIM 360 hub — resolve by name to UUID
                let matched = roles
                    .iter()
                    .find(|r| r.name == role)
                    .or_else(|| {
                        roles
                            .iter()
                            .find(|r| r.name.to_lowercase() == role.to_lowercase())
                    })
                    .or_else(|| {
                        roles
                            .iter()
                            .find(|r| r.name.to_lowercase().contains(&role.to_lowercase()))
                    });

                if let Some(r) = matched {
                    return Ok(ResolvedRole::Uuid(r.id.clone()));
                }

                // Name not found in BIM 360 roles — try ACC products as fallback
                if let Some(products) = role_name_to_acc_products(role) {
                    return Ok(ResolvedRole::Products(products));
                }

                let available: Vec<String> =
                    roles.iter().map(|r| format!("\"{}\"", r.name)).collect();
                anyhow::bail!(
                    "Role {:?} not found. Available roles: {}",
                    role,
                    available.join(", ")
                )
            }
            _ => {
                // ACC hub (or roles endpoint unavailable) — use product-based mapping
                if let Some(products) = role_name_to_acc_products(role) {
                    return Ok(ResolvedRole::Products(products));
                }
                anyhow::bail!(
                    "Role {:?} not recognized. Known ACC roles: \
                    \"Project Admin\", \"Project Member\", \"Project Editor\", \"Project Viewer\"",
                    role
                )
            }
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

    async fn list_roles_bim360(
        &self,
        account_id: &str,
        project_id: &str,
    ) -> Result<Vec<AccountRole>> {
        let token = self.auth.get_3leg_token().await?;
        // BIM 360 industry roles are project-level:
        // GET /hq/v2/accounts/:account_id/projects/:project_id/industry_roles
        let url = format!(
            "{}/projects/{}/industry_roles",
            self.hq_v2_url(account_id),
            project_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list BIM 360 industry roles (HTTP {status}): {body}");
        }

        // BIM 360 v2 industry_roles returns plain array of role objects
        let roles: Vec<Bim360Role> = response
            .json()
            .await
            .context("Failed to parse BIM 360 industry roles response")?;

        Ok(roles
            .into_iter()
            .map(|r| AccountRole {
                id: r.id,
                name: r.name,
            })
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

/// Map a well-known role display name to ACC product access configurations.
///
/// ACC does not have a `role_id` concept — access is controlled via product keys.
/// Returns `None` for unrecognized role names.
fn role_name_to_acc_products(role: &str) -> Option<Vec<ProductAccess>> {
    let key = |k: &str, a: &str| ProductAccess {
        key: k.to_string(),
        access: a.to_string(),
    };
    match role.to_lowercase().trim() {
        "project admin" | "admin" | "administrator" => Some(vec![
            key("projectAdministration", "administrator"),
            key("docs", "administrator"),
        ]),
        // ACC API rule: projectAdministration cannot be "member" — only "administrator" or "none".
        // When set to "none", all other products must use "member" access.
        "project member" | "member" => Some(vec![
            key("projectAdministration", "none"),
            key("docs", "member"),
        ]),
        "project editor" | "editor" => Some(vec![
            key("projectAdministration", "none"),
            key("docs", "editor"),
        ]),
        "project viewer" | "viewer" => Some(vec![
            key("projectAdministration", "none"),
            key("docs", "viewer"),
        ]),
        _ => None,
    }
}

/// Returns true if the string looks like a UUID (with or without dashes)
fn is_uuid(s: &str) -> bool {
    let s = s.replace('-', "");
    s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit())
}
