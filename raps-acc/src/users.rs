// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project Users API client for ACC/BIM 360

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use serde::Serialize;
use tokio::sync::Semaphore;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::{self, HttpClientConfig};

use crate::types::{PaginatedResponse, ProductAccess, ProjectUser};

/// Client for ACC Project Users API
///
/// Provides operations for managing users within individual projects.
/// Set `account_id` to enable BIM 360 HQ v2 fallback for Business hubs.
pub struct ProjectUsersClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
    /// Account ID for BIM 360 HQ v2 user endpoints (required for Business hubs)
    pub account_id: Option<String>,
    /// Learned: ACC endpoint returns 500 for this account → skip probe, go straight to BIM 360
    acc_probe_failed: Arc<AtomicBool>,
}

/// Request to add a user to a project
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectUserRequest {
    /// User email address
    pub email: String,
    /// Role IDs to assign (ACC API uses an array, even for a single role)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<String>,
    /// Product access configurations (ACC requires this field even when empty)
    pub products: Vec<ProductAccess>,
    /// Company ID to assign (ACC uses companyId UUID; serialized as camelCase)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    /// Suppress project invite email to the user (default: false)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub suppress_administrative_emails: bool,
    /// Product keys activated on the target project (e.g. `["docs", "projectAdministration"]`).
    /// Used by the BIM 360 import path to avoid sending services the project doesn't have.
    /// `None` means unknown — the BIM 360 path will fetch the project info.
    #[serde(skip)]
    pub project_product_keys: Option<Vec<String>>,
    /// Platform hint from the project listing (e.g. `"acc"` or `"bim360"`).
    /// When set, skips the ACC probe and goes directly to the correct endpoint.
    #[serde(skip)]
    pub platform: Option<String>,
    /// Company name for BIM 360 import (BIM 360 uses company name, not UUID).
    /// If set, used in the BIM 360 HQ v2 import path.
    #[serde(skip)]
    pub company_name: Option<String>,
}

// ── BIM 360 HQ v2 /users/import types ────────────────────────────────────────

/// Single entry in a BIM 360 HQ v2 `users/import` request array.
/// Uses snake_case fields (BIM 360 does NOT use camelCase).
#[derive(Debug, Clone, Serialize)]
struct Bim360ImportEntry {
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<String>,
    services: Bim360ImportServices,
    industry_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Bim360ImportServices {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_administration: Option<Bim360ServiceAccess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_management: Option<Bim360ServiceAccess>,
}

#[derive(Debug, Clone, Serialize)]
struct Bim360ServiceAccess {
    access_level: String,
}

/// Response from BIM 360 HQ v2 `users/import`
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Bim360ImportResponse {
    success: u32,
    failure: u32,
    success_items: Vec<Bim360ImportItem>,
    failure_items: Vec<Bim360ImportFailureItem>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Bim360ImportItem {
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Bim360ImportFailureItem {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    errors: Vec<Bim360ImportError>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Bim360ImportError {
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    code: Option<serde_json::Value>,
}

impl Bim360ImportEntry {
    /// Build from an `AddProjectUserRequest`, mapping ACC products → BIM 360 services.
    ///
    /// - `products` non-empty: translate `projectAdministration.administrator` to admin,
    ///   everything else to user
    /// - `role_ids` non-empty: these are BIM 360 industry role UUIDs — put them in
    ///   `industry_roles`, default to `document_management: user`
    /// - Neither: default to `document_management: user`
    ///
    /// Only includes services that are actually activated on the target project.
    /// BIM 360 returns `nil:NilClass` if sent a service the project doesn't have.
    fn from_request(req: &AddProjectUserRequest, project_product_keys: &[String]) -> Self {
        let is_admin = req
            .products
            .iter()
            .any(|p| p.key == "projectAdministration" && p.access == "administrator");

        // Only include services the project actually has.
        // BIM 360 product keys: "docs" = Document Management, "projectAdministration" = Project Admin.
        // An empty product list means we don't know → include all services (safe default
        // for most BIM 360 projects which do have Document Management).
        let has_doc_management = project_product_keys.is_empty()
            || project_product_keys.iter().any(|k| {
                let lower = k.to_lowercase();
                lower == "docs" || lower == "document_management" || lower == "documentmanagement"
            });

        let doc_management = if has_doc_management {
            let level = if is_admin { "admin" } else { "user" };
            Some(Bim360ServiceAccess {
                access_level: level.to_string(),
            })
        } else {
            None
        };

        let services = Bim360ImportServices {
            project_administration: if is_admin {
                Some(Bim360ServiceAccess {
                    access_level: "admin".to_string(),
                })
            } else {
                None
            },
            document_management: doc_management,
        };

        Bim360ImportEntry {
            email: req.email.clone(),
            company: req.company_name.clone(),
            services,
            industry_roles: req.role_ids.clone(),
        }
    }
}

/// Request to update a project user
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectUserRequest {
    /// New role IDs to assign (ACC API uses an array, even for a single role)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<String>,
    /// Updated product access configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<ProductAccess>>,
}

/// User import request item
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUserRequest {
    /// User email address
    pub email: String,
    /// Role IDs to assign (ACC API uses an array, even for a single role)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub role_ids: Vec<String>,
    /// Optional product access configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<ProductAccess>>,
}

/// Result of a bulk user import operation
#[derive(Debug, Clone)]
pub struct ImportUsersResult {
    /// Total number of users attempted
    pub total: usize,
    /// Number of users successfully imported
    pub imported: usize,
    /// Number of users that failed to import
    pub failed: usize,
    /// Individual errors for failed imports
    pub errors: Vec<ImportUserError>,
    /// Successfully imported users
    pub successes: Vec<ImportUserSuccess>,
}

/// Error details for a failed user import
#[derive(Debug, Clone)]
pub struct ImportUserError {
    /// Email of the user that failed to import
    pub email: String,
    /// Error message describing why the import failed
    pub error: String,
}

/// Success details for an imported user
#[derive(Debug, Clone)]
pub struct ImportUserSuccess {
    /// Email of the successfully imported user
    pub email: String,
    /// User ID if available
    pub user_id: Option<String>,
}

impl Clone for ProjectUsersClient {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            auth: self.auth.clone(),
            http_client: self.http_client.clone(),
            account_id: self.account_id.clone(),
            acc_probe_failed: Arc::clone(&self.acc_probe_failed),
        }
    }
}

impl ProjectUsersClient {
    /// Create a new Project Users client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
            .expect("default HTTP client configuration must always succeed")
    }

    /// Create client with custom HTTP configuration.
    ///
    /// Returns an error if the HTTP client cannot be built (e.g. invalid proxy URL).
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> anyhow::Result<Self> {
        let http_client = http_config
            .create_client()
            .context("Failed to initialise HTTP client for Project Users")?;

        Ok(Self {
            config,
            auth,
            http_client,
            account_id: None,
            acc_probe_failed: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get the base URL for ACC Construction Admin v1 project endpoint
    fn project_url(&self, project_id: &str) -> String {
        let project_id = crate::strip_project_prefix(project_id);
        format!(
            "{}/construction/admin/v1/projects/{}",
            self.config.base_url, project_id
        )
    }

    /// Get the base URL for a BIM 360 HQ v2 project (no trailing path segment)
    fn project_url_bim360(&self, account_id: &str, project_id: &str) -> String {
        let project_id = crate::strip_project_prefix(project_id);
        format!(
            "{}/hq/v2/accounts/{}/projects/{}",
            self.config.base_url, account_id, project_id
        )
    }

    /// List members of a project (paginated)
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `limit` - Maximum results per page (max: 100)
    /// * `offset` - Starting index
    pub async fn list_project_users(
        &self,
        project_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<ProjectUser>> {
        // Fast path: skip ACC probe if we already know this is a BIM 360 account.
        if self.acc_probe_failed.load(Ordering::Relaxed)
            && let Some(ref account_id) = self.account_id
        {
            return self
                .list_project_users_bim360(account_id, project_id, limit, offset)
                .await;
        }

        let token = self.auth.get_3leg_token().await?;

        let mut url = format!("{}/users", self.project_url(project_id));

        // Build query parameters
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l.min(100)));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        // Single probe — don't retry on 400/404 (fall back to BIM 360 instead).
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to send ACC list-project-users request")?;

        if response.status().is_success() {
            let users_response: PaginatedResponse<ProjectUser> = response
                .json()
                .await
                .context("Failed to parse project users response")?;
            return Ok(users_response);
        }

        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_default();

        // On 400/404 try BIM 360 HQ v2 if we have an account_id.
        if (status == 400 || status == 404)
            && let Some(ref account_id) = self.account_id
        {
            if status == 404 {
                self.acc_probe_failed.store(true, Ordering::Relaxed);
            }
            return self
                .list_project_users_bim360(account_id, project_id, limit, offset)
                .await;
        }

        anyhow::bail!("Failed to list project users (HTTP {status}): {error_text}");
    }

    /// List project users via BIM 360 HQ v2 endpoint.
    ///
    /// HQ v2 returns a plain JSON array (not a paginated wrapper), so we
    /// construct a `PaginatedResponse` from the result.
    async fn list_project_users_bim360(
        &self,
        account_id: &str,
        project_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<ProjectUser>> {
        let token = self.auth.get_3leg_token().await?;

        let mut url = format!("{}/users", self.project_url_bim360(account_id, project_id));

        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l.min(100)));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();

        if !(200..300).contains(&(status as usize)) {
            anyhow::bail!("Failed to list BIM 360 project users (HTTP {status}): {body_text}");
        }

        // BIM 360 HQ v2 returns a plain JSON array with snake_case fields.
        let arr: Vec<serde_json::Value> = serde_json::from_str(&body_text)
            .context("Failed to parse BIM 360 project users response")?;

        let effective_limit = limit.map(|l| l.min(100)).unwrap_or(100);
        let effective_offset = offset.unwrap_or(0);
        let count = arr.len();

        let users: Vec<ProjectUser> = arr
            .into_iter()
            .map(|v| ProjectUser {
                id: v["user_id"].as_str().unwrap_or_default().to_string(),
                email: v["email"].as_str().map(|s| s.to_string()),
                name: v["name"].as_str().map(|s| s.to_string()),
                role_ids: v["industry_roles"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|r| r.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                role_name: None,
                products: None,
                added_on: None,
            })
            .collect();

        // BIM 360 HQ v2 doesn't return total count in the body; approximate
        // based on whether a full page was returned.
        let total_results = if count < effective_limit {
            effective_offset + count
        } else {
            // There may be more — use a sentinel that makes has_more() return true
            effective_offset + count + 1
        };

        Ok(PaginatedResponse {
            results: users,
            pagination: crate::types::PaginationInfo {
                limit: effective_limit,
                offset: effective_offset,
                total_results,
            },
        })
    }

    /// Get a specific user's membership in a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `user_id` - The user ID
    pub async fn get_project_user(&self, project_id: &str, user_id: &str) -> Result<ProjectUser> {
        // Fast path: skip ACC probe if we already know this is a BIM 360 account.
        if self.acc_probe_failed.load(Ordering::Relaxed)
            && let Some(ref account_id) = self.account_id
        {
            return self
                .get_project_user_bim360(account_id, project_id, user_id)
                .await;
        }

        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

        // Single probe — don't retry on 400/404 (fall back to BIM 360 instead).
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to send ACC get-project-user request")?;

        if response.status().is_success() {
            return response
                .json()
                .await
                .context("Failed to parse project user response");
        }

        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_default();

        // On 400/404 try BIM 360 HQ v2 if we have an account_id.
        if (status == 400 || status == 404)
            && let Some(ref account_id) = self.account_id
        {
            if status == 404 {
                self.acc_probe_failed.store(true, Ordering::Relaxed);
            }
            return self
                .get_project_user_bim360(account_id, project_id, user_id)
                .await;
        }

        anyhow::bail!("Failed to get project user (HTTP {status}): {error_text}");
    }

    /// Get a project user via BIM 360 HQ v2 endpoint.
    async fn get_project_user_bim360(
        &self,
        account_id: &str,
        project_id: &str,
        user_id: &str,
    ) -> Result<ProjectUser> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users/{}",
            self.project_url_bim360(account_id, project_id),
            user_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();

        if !(200..300).contains(&(status as usize)) {
            anyhow::bail!("Failed to get BIM 360 project user (HTTP {status}): {body_text}");
        }

        // BIM 360 HQ v2 returns snake_case fields — parse manually.
        let v: serde_json::Value = serde_json::from_str(&body_text)
            .context("Failed to parse BIM 360 project user response")?;

        Ok(ProjectUser {
            id: v["user_id"].as_str().unwrap_or_default().to_string(),
            email: v["email"].as_str().map(|s| s.to_string()),
            name: v["name"].as_str().map(|s| s.to_string()),
            role_ids: v["industry_roles"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| r.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            role_name: None,
            products: None,
            added_on: None,
        })
    }

    /// Add a user to a project.
    ///
    /// Tries the ACC Construction Admin v1 endpoint first. If the project lives
    /// in a BIM 360 Business hub (returns HTTP 400 or 404) and `self.account_id`
    /// is set, falls back to the BIM 360 HQ v2 endpoint.
    pub async fn add_user(
        &self,
        project_id: &str,
        request: AddProjectUserRequest,
    ) -> Result<ProjectUser> {
        // Fast path: if the caller knows the platform, skip the probe entirely.
        if let Some(ref platform) = request.platform
            && platform == "bim360"
            && let Some(ref account_id) = self.account_id
        {
            return self.add_user_bim360(account_id, project_id, request).await;
        }

        // Cached fast path: if we've learned that ACC returns 404 for this
        // account (no platform hint available), go straight to BIM 360.
        if self.acc_probe_failed.load(Ordering::Relaxed)
            && let Some(ref account_id) = self.account_id
        {
            return self.add_user_bim360(account_id, project_id, request).await;
        }

        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/users", self.project_url(project_id));

        // Send a single probe to the ACC endpoint — do NOT use send_with_retry
        // because 400/404/500 should fall back to BIM 360 immediately, not retry
        // with exponential backoff (which wastes 60+s on BIM 360 accounts).
        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send ACC add-user request")?;

        if response.status().is_success() {
            return response
                .json()
                .await
                .context("Failed to parse add user response");
        }

        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_default();

        // On 400/404/500 try BIM 360 HQ v2 if we have an account_id.
        // 404 occurs when the project is BIM 360 (ACC endpoint doesn't know it).
        // 400/500 can also indicate a BIM 360 project, but 500 with messages like
        // "reqBodyProducts is not iterable" is a request validation bug — don't
        // cache that as a blanket "ACC never works" signal.
        if (status == 400 || status == 404 || status == 500)
            && let Some(ref account_id) = self.account_id
        {
            // Only cache the probe failure for errors that strongly indicate
            // the entire account is BIM 360 (404 = endpoint not found for project).
            // Do NOT cache 500 — it may be a transient server bug or a request
            // validation error (e.g. missing products field) that affects one
            // request, not the whole account.
            if status == 404 {
                self.acc_probe_failed.store(true, Ordering::Relaxed);
            }
            return self.add_user_bim360(account_id, project_id, request).await;
        }

        anyhow::bail!("Failed to add user to project (HTTP {status}): {error_text}");
    }

    /// Fetch a BIM 360 project's activated product keys via ACC Admin v1.
    ///
    /// Returns product keys like `["docs", "projectAdministration", "insight"]`.
    /// On failure (e.g. 404), returns an empty vec (caller should treat as unknown).
    async fn fetch_project_product_keys(&self, project_id: &str) -> Vec<String> {
        let Ok(token) = self.auth.get_3leg_token().await else {
            return vec![];
        };
        let url = self.project_url(project_id).to_string();

        let Ok(response) = self.http_client.get(&url).bearer_auth(&token).send().await else {
            return vec![];
        };
        if !response.status().is_success() {
            return vec![];
        }
        let Ok(body) = response.json::<serde_json::Value>().await else {
            return vec![];
        };
        body.get("products")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| p.get("key").and_then(|k| k.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Add a user to a BIM 360 project via HQ v2 `/users/import` endpoint.
    ///
    /// BIM 360 differs from ACC:
    /// - Endpoint: `POST /hq/v2/accounts/{account}/projects/{project}/users/import`
    /// - Request: array of `{ email, services: { document_management: { access_level } }, industry_roles: [] }`
    /// - Response: `{ success, failure, success_items, failure_items }`
    /// - 404: user not in BIM 360 account (must be added via Account Admin first)
    async fn add_user_bim360(
        &self,
        account_id: &str,
        project_id: &str,
        request: AddProjectUserRequest,
    ) -> Result<ProjectUser> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users/import",
            self.project_url_bim360(account_id, project_id)
        );

        // Determine which services the project supports. If the caller already
        // provided product keys (bulk path), use them; otherwise fetch.
        let product_keys = match &request.project_product_keys {
            Some(keys) if !keys.is_empty() => keys.clone(),
            _ => self.fetch_project_product_keys(project_id).await,
        };

        let entry = Bim360ImportEntry::from_request(&request, &product_keys);
        let body = vec![entry];

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&body)
        })
        .await?;

        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();

        if status == 404 {
            anyhow::bail!(
                "User '{}' not found in BIM 360 account. \
                 Add the user to the account in BIM 360 Account Admin → Members first.",
                request.email
            );
        }

        if !(200..300).contains(&(status as usize)) {
            anyhow::bail!("Failed to add user to BIM 360 project (HTTP {status}): {body_text}");
        }

        let result: Bim360ImportResponse =
            serde_json::from_str(&body_text).context("Failed to parse BIM 360 import response")?;

        if result.success > 0 {
            let item = result
                .success_items
                .into_iter()
                .next()
                .unwrap_or(Bim360ImportItem {
                    user_id: None,
                    email: Some(request.email.clone()),
                });
            return Ok(crate::types::ProjectUser {
                id: item.user_id.unwrap_or_default(),
                email: item.email.or(Some(request.email)),
                name: None,
                role_ids: vec![],
                role_name: None,
                products: None,
                added_on: None,
            });
        }

        // Report failure details
        let error_msg = result
            .failure_items
            .into_iter()
            .flat_map(|fi| fi.errors.into_iter().filter_map(|e| e.message))
            .collect::<Vec<_>>()
            .join("; ");

        anyhow::bail!(
            "BIM 360 failed to add user '{}' to project: {}",
            request.email,
            if error_msg.is_empty() {
                "unknown error".to_string()
            } else {
                error_msg
            }
        )
    }

    /// Update a user's role or product access in a project.
    ///
    /// Tries the ACC Construction Admin v1 endpoint first. On 400 (platform
    /// mismatch) falls back to BIM 360 HQ v2 PATCH if `account_id` is set.
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `user_id` - The user ID to update
    /// * `request` - Update request with new role or products
    pub async fn update_user(
        &self,
        project_id: &str,
        user_id: &str,
        request: UpdateProjectUserRequest,
    ) -> Result<ProjectUser> {
        // Fast path: skip ACC probe if we already know this is a BIM 360 account.
        if self.acc_probe_failed.load(Ordering::Relaxed)
            && let Some(ref account_id) = self.account_id
        {
            return self
                .update_user_bim360(account_id, project_id, user_id, &request)
                .await;
        }

        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send ACC update-user request")?;

        let status = response.status().as_u16();

        if response.status().is_success() {
            match response.json::<ProjectUser>().await {
                Ok(user) => return Ok(user),
                Err(_) if self.account_id.is_some() => {
                    // ACC endpoint returned 200 but response doesn't match ProjectUser —
                    // likely a BIM 360 project routed through ACC. Fall through to BIM 360.
                    self.acc_probe_failed.store(true, Ordering::Relaxed);
                    return self
                        .update_user_bim360(
                            self.account_id.as_ref().unwrap(),
                            project_id,
                            user_id,
                            &request,
                        )
                        .await;
                }
                Err(e) => {
                    return Err(e).context("Failed to parse update user response");
                }
            }
        }

        // On 400/500 try BIM 360 HQ v2 if we have an account_id.
        if (status == 400 || status == 500)
            && let Some(ref account_id) = self.account_id
        {
            return self
                .update_user_bim360(account_id, project_id, user_id, &request)
                .await;
        }

        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to update project user (HTTP {status}): {error_text}");
    }

    /// Update a user in a BIM 360 project via HQ v2 PATCH endpoint.
    async fn update_user_bim360(
        &self,
        account_id: &str,
        project_id: &str,
        user_id: &str,
        request: &UpdateProjectUserRequest,
    ) -> Result<ProjectUser> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users/{}",
            self.project_url_bim360(account_id, project_id),
            user_id
        );

        // BIM 360 HQ v2 uses snake_case: { "industry_roles": [...] }
        // Map role_ids → industry_roles for BIM 360
        let body = serde_json::json!({
            "industry_roles": request.role_ids,
        });

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&body)
        })
        .await?;

        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();

        if !(200..300).contains(&(status as usize)) {
            anyhow::bail!("Failed to update BIM 360 project user (HTTP {status}): {body_text}");
        }

        // BIM 360 HQ v2 returns snake_case fields (user_id, account_id, etc.)
        // which don't match ProjectUser's camelCase expectations. Parse manually.
        let v: serde_json::Value = serde_json::from_str(&body_text)
            .context("Failed to parse BIM 360 update user response")?;

        Ok(ProjectUser {
            id: v["user_id"].as_str().unwrap_or_default().to_string(),
            email: v["email"].as_str().map(|s| s.to_string()),
            name: None,
            role_ids: v["industry_roles"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| r.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            role_name: None,
            products: None,
            added_on: None,
        })
    }

    /// Remove a user from a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `user_id` - The user ID to remove
    pub async fn remove_user(&self, project_id: &str, user_id: &str) -> Result<()> {
        // Fast path: skip ACC probe if we already know this is a BIM 360 account.
        if self.acc_probe_failed.load(Ordering::Relaxed)
            && let Some(ref account_id) = self.account_id
        {
            return self
                .remove_user_bim360(account_id, project_id, user_id)
                .await;
        }

        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

        // Single probe — don't retry on 400/404 (fall back to BIM 360 instead).
        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to send ACC remove-user request")?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_default();

        // On 400/404 try BIM 360 HQ v2 if we have an account_id.
        if (status == 400 || status == 404)
            && let Some(ref account_id) = self.account_id
        {
            if status == 404 {
                self.acc_probe_failed.store(true, Ordering::Relaxed);
            }
            return self
                .remove_user_bim360(account_id, project_id, user_id)
                .await;
        }

        anyhow::bail!("Failed to remove user from project (HTTP {status}): {error_text}");
    }

    /// Remove a user from a BIM 360 project via HQ v2 DELETE endpoint.
    async fn remove_user_bim360(
        &self,
        account_id: &str,
        project_id: &str,
        user_id: &str,
    ) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users/{}",
            self.project_url_bim360(account_id, project_id),
            user_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to remove user from BIM 360 project ({status}): {error_text}");
        }

        Ok(())
    }

    /// Check if a user exists in a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `user_id` - The user ID to check
    ///
    /// # Returns
    /// True if the user is a member of the project, false otherwise
    pub async fn user_exists(&self, project_id: &str, user_id: &str) -> Result<bool> {
        // Fast path: skip ACC probe if we already know this is a BIM 360 account.
        if self.acc_probe_failed.load(Ordering::Relaxed)
            && let Some(ref account_id) = self.account_id
        {
            return self
                .user_exists_bim360(account_id, project_id, user_id)
                .await;
        }

        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

        // Single probe — don't retry on 400/404 (fall back to BIM 360 instead).
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to send ACC user-exists request")?;

        if response.status().is_success() {
            return Ok(true);
        }

        let status = response.status().as_u16();

        // On 400/404 try BIM 360 HQ v2 if we have an account_id.
        if (status == 400 || status == 404)
            && let Some(ref account_id) = self.account_id
        {
            if status == 404 {
                self.acc_probe_failed.store(true, Ordering::Relaxed);
            }
            return self
                .user_exists_bim360(account_id, project_id, user_id)
                .await;
        }

        Ok(false)
    }

    /// Check if a user exists in a BIM 360 project via HQ v2 endpoint.
    async fn user_exists_bim360(
        &self,
        account_id: &str,
        project_id: &str,
        user_id: &str,
    ) -> Result<bool> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users/{}",
            self.project_url_bim360(account_id, project_id),
            user_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        Ok(response.status().is_success())
    }

    /// Find a user in a project by email address.
    ///
    /// Uses `filter[email]` query parameter to avoid fetching all users.
    /// Returns `None` if the user is not a member of the project.
    pub async fn find_project_user_by_email(
        &self,
        project_id: &str,
        email: &str,
    ) -> Result<Option<ProjectUser>> {
        // Fast path: skip ACC probe if we already know this is a BIM 360 account.
        if self.acc_probe_failed.load(Ordering::Relaxed)
            && let Some(ref account_id) = self.account_id
        {
            return self
                .find_project_user_by_email_bim360(account_id, project_id, email)
                .await;
        }

        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users?filter[email]={}&limit=1",
            self.project_url(project_id),
            urlencoding::encode(email),
        );

        // Single probe — don't retry on 400/404 (fall back to BIM 360 instead).
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to send ACC find-user-by-email request")?;

        if response.status().is_success() {
            let page: PaginatedResponse<ProjectUser> = response
                .json()
                .await
                .context("Failed to parse project users response")?;
            return Ok(page.results.into_iter().next());
        }

        let status = response.status().as_u16();
        let error_text = response.text().await.unwrap_or_default();

        // On 400/404 try BIM 360 HQ v2 if we have an account_id.
        if (status == 400 || status == 404)
            && let Some(ref account_id) = self.account_id
        {
            if status == 404 {
                self.acc_probe_failed.store(true, Ordering::Relaxed);
            }
            return self
                .find_project_user_by_email_bim360(account_id, project_id, email)
                .await;
        }

        anyhow::bail!("Failed to find project user by email (HTTP {status}): {error_text}");
    }

    /// Find a project user by email via BIM 360 HQ v2 endpoint.
    ///
    /// BIM 360 HQ v2 may not support `filter[email]`, so we fetch the first
    /// page of users (limit=100) and filter in memory.
    async fn find_project_user_by_email_bim360(
        &self,
        account_id: &str,
        project_id: &str,
        email: &str,
    ) -> Result<Option<ProjectUser>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users?limit=100",
            self.project_url_bim360(account_id, project_id)
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();

        if !(200..300).contains(&(status as usize)) {
            anyhow::bail!(
                "Failed to find BIM 360 project user by email (HTTP {status}): {body_text}"
            );
        }

        // BIM 360 HQ v2 returns a plain JSON array with snake_case fields.
        let arr: Vec<serde_json::Value> = serde_json::from_str(&body_text)
            .context("Failed to parse BIM 360 project users response")?;

        let email_lower = email.to_lowercase();
        let found = arr.into_iter().find(|v| {
            v["email"]
                .as_str()
                .map(|e| e.to_lowercase() == email_lower)
                .unwrap_or(false)
        });

        Ok(found.map(|v| ProjectUser {
            id: v["user_id"].as_str().unwrap_or_default().to_string(),
            email: v["email"].as_str().map(|s| s.to_string()),
            name: v["name"].as_str().map(|s| s.to_string()),
            role_ids: v["industry_roles"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|r| r.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            role_name: None,
            products: None,
            added_on: None,
        }))
    }

    /// Fetch all users in a project (handles pagination automatically)
    pub async fn list_all_project_users(&self, project_id: &str) -> Result<Vec<ProjectUser>> {
        let mut all_users = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let response = self
                .list_project_users(project_id, Some(limit), Some(offset))
                .await?;
            let has_more = response.has_more();
            let next_offset = response.next_offset();
            all_users.extend(response.results);

            if !has_more {
                break;
            }
            offset = next_offset;
        }

        Ok(all_users)
    }

    /// Import multiple users to a project concurrently
    ///
    /// Adds each user individually via concurrent requests bounded by a semaphore
    /// (max 10 concurrent) for rate-limit safety. Collects per-user results.
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `users` - List of users to import
    ///
    /// # Returns
    /// An `ImportUsersResult` containing the overall summary and individual results
    pub async fn import_users(
        &self,
        project_id: &str,
        users: Vec<ImportUserRequest>,
    ) -> Result<ImportUsersResult> {
        let total = users.len();
        let semaphore = Arc::new(Semaphore::new(10));
        let mut join_set = tokio::task::JoinSet::new();

        for user in users {
            let client = self.clone();
            let sem = semaphore.clone();
            let pid = project_id.to_string();
            let email = user.email.clone();

            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed unexpectedly");
                let request = AddProjectUserRequest {
                    email: user.email.clone(),
                    role_ids: user.role_ids,
                    products: user.products.unwrap_or_default(),
                    company_id: None,
                    suppress_administrative_emails: false,
                    project_product_keys: None,
                    platform: None,
                    company_name: None,
                };
                let result = client.add_user(&pid, request).await;
                (email, result)
            });
        }

        let mut imported = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        let mut successes = Vec::new();

        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok((email, Ok(project_user))) => {
                    imported += 1;
                    successes.push(ImportUserSuccess {
                        email,
                        user_id: Some(project_user.id),
                    });
                }
                Ok((email, Err(e))) => {
                    failed += 1;
                    errors.push(ImportUserError {
                        email,
                        error: e.to_string(),
                    });
                }
                Err(e) => {
                    failed += 1;
                    errors.push(ImportUserError {
                        email: "unknown".to_string(),
                        error: format!("Task join error: {e}"),
                    });
                }
            }
        }

        Ok(ImportUsersResult {
            total,
            imported,
            failed,
            errors,
            successes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_request_role_ids_absent_when_empty() {
        let request = AddProjectUserRequest {
            email: "user@example.com".to_string(),
            role_ids: vec![],
            products: vec![],
            company_id: None,
            suppress_administrative_emails: false,
            project_product_keys: None,
            platform: None,
            company_name: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"email\":\"user@example.com\""));
        assert!(
            !json.contains("roleIds"),
            "roleIds must be absent when empty (skip_serializing_if)"
        );
    }

    #[test]
    fn test_add_request_serialization() {
        let request = AddProjectUserRequest {
            email: "user@example.com".to_string(),
            role_ids: vec!["role-456".to_string()],
            products: vec![ProductAccess {
                key: "docs".to_string(),
                access: "member".to_string(),
            }],
            company_id: None,
            suppress_administrative_emails: false,
            project_product_keys: None,
            platform: None,
            company_name: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"email\":\"user@example.com\""));
        assert!(
            json.contains("\"roleIds\":[\"role-456\"]"),
            "must send roleIds array: {json}"
        );
        assert!(json.contains("docs"));
    }

    #[test]
    fn test_update_request_serialization() {
        let request = UpdateProjectUserRequest {
            role_ids: vec!["new-role".to_string()],
            products: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(
            json.contains("\"roleIds\":[\"new-role\"]"),
            "must send roleIds array: {json}"
        );
        // products should be skipped when None
        assert!(!json.contains("products"));
    }

    #[test]
    fn test_bim360_import_omits_doc_management_when_project_lacks_it() {
        let request = AddProjectUserRequest {
            email: "user@example.com".to_string(),
            role_ids: vec![],
            products: vec![
                ProductAccess {
                    key: "projectAdministration".to_string(),
                    access: "administrator".to_string(),
                },
                ProductAccess {
                    key: "docs".to_string(),
                    access: "administrator".to_string(),
                },
            ],
            company_id: None,
            suppress_administrative_emails: false,
            project_product_keys: None,
            platform: None,
            company_name: None,
        };

        // Project without Document Management (only projectAdministration + insight)
        let keys = vec!["projectAdministration".to_string(), "insight".to_string()];
        let entry = Bim360ImportEntry::from_request(&request, &keys);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            !json.contains("document_management"),
            "must omit document_management when project lacks it: {json}"
        );
        assert!(
            json.contains("project_administration"),
            "must include project_administration: {json}"
        );
    }

    #[test]
    fn test_bim360_import_includes_doc_management_when_project_has_it() {
        let request = AddProjectUserRequest {
            email: "user@example.com".to_string(),
            role_ids: vec![],
            products: vec![
                ProductAccess {
                    key: "projectAdministration".to_string(),
                    access: "administrator".to_string(),
                },
                ProductAccess {
                    key: "docs".to_string(),
                    access: "administrator".to_string(),
                },
            ],
            company_id: None,
            suppress_administrative_emails: false,
            project_product_keys: None,
            platform: None,
            company_name: None,
        };

        // Project WITH Document Management
        let keys = vec![
            "projectAdministration".to_string(),
            "docs".to_string(),
            "insight".to_string(),
        ];
        let entry = Bim360ImportEntry::from_request(&request, &keys);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains("document_management"),
            "must include document_management: {json}"
        );
        assert!(
            json.contains("project_administration"),
            "must include project_administration: {json}"
        );
    }

    #[test]
    fn test_bim360_import_includes_doc_management_when_keys_unknown() {
        let request = AddProjectUserRequest {
            email: "user@example.com".to_string(),
            role_ids: vec![],
            products: vec![],
            company_id: None,
            suppress_administrative_emails: false,
            project_product_keys: None,
            platform: None,
            company_name: None,
        };

        // Empty product keys = unknown → safe default: include document_management
        let entry = Bim360ImportEntry::from_request(&request, &[]);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains("document_management"),
            "must default to including document_management: {json}"
        );
    }

    #[test]
    fn test_import_users_result_aggregation() {
        let result = ImportUsersResult {
            total: 5,
            imported: 3,
            failed: 2,
            errors: vec![
                ImportUserError {
                    email: "bad1@test.com".to_string(),
                    error: "Not found".to_string(),
                },
                ImportUserError {
                    email: "bad2@test.com".to_string(),
                    error: "Conflict".to_string(),
                },
            ],
            successes: vec![
                ImportUserSuccess {
                    email: "ok1@test.com".to_string(),
                    user_id: Some("u1".to_string()),
                },
                ImportUserSuccess {
                    email: "ok2@test.com".to_string(),
                    user_id: Some("u2".to_string()),
                },
                ImportUserSuccess {
                    email: "ok3@test.com".to_string(),
                    user_id: Some("u3".to_string()),
                },
            ],
        };

        assert_eq!(result.total, 5);
        assert_eq!(result.imported + result.failed, result.total);
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.successes.len(), 3);
        assert_eq!(result.errors[0].email, "bad1@test.com");
        assert_eq!(result.successes[0].email, "ok1@test.com");
    }
}
