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
    /// Product access configurations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub products: Vec<ProductAccess>,
    /// Suppress project invite email to the user (default: false)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub suppress_administrative_emails: bool,
}

// ── BIM 360 HQ v2 /users/import types ────────────────────────────────────────

/// Single entry in a BIM 360 HQ v2 `users/import` request array.
/// Uses snake_case fields (BIM 360 does NOT use camelCase).
#[derive(Debug, Clone, Serialize)]
struct Bim360ImportEntry {
    email: String,
    services: Bim360ImportServices,
    industry_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Bim360ImportServices {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_administration: Option<Bim360ServiceAccess>,
    document_management: Bim360ServiceAccess,
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
    fn from_request(req: &AddProjectUserRequest) -> Self {
        let is_admin = req.products.iter().any(|p| {
            p.key == "projectAdministration" && p.access == "administrator"
        });

        let services = if is_admin {
            Bim360ImportServices {
                project_administration: Some(Bim360ServiceAccess { access_level: "admin".to_string() }),
                document_management: Bim360ServiceAccess { access_level: "admin".to_string() },
            }
        } else {
            Bim360ImportServices {
                project_administration: None,
                document_management: Bim360ServiceAccess { access_level: "user".to_string() },
            }
        };

        Bim360ImportEntry {
            email: req.email.clone(),
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
    /// * `limit` - Maximum results per page (max: 200)
    /// * `offset` - Starting index
    pub async fn list_project_users(
        &self,
        project_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<ProjectUser>> {
        let token = self.auth.get_3leg_token().await?;

        let mut url = format!("{}/users", self.project_url(project_id));

        // Build query parameters
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l.min(200)));
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

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list project users ({status}): {error_text}");
        }

        let users_response: PaginatedResponse<ProjectUser> = response
            .json()
            .await
            .context("Failed to parse project users response")?;

        Ok(users_response)
    }

    /// Get a specific user's membership in a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `user_id` - The user ID
    pub async fn get_project_user(&self, project_id: &str, user_id: &str) -> Result<ProjectUser> {
        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project user ({status}): {error_text}");
        }

        let user: ProjectUser = response
            .json()
            .await
            .context("Failed to parse project user response")?;

        Ok(user)
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
        // Fast path: if we've already learned that ACC returns 500 for this
        // account, skip the probe and go straight to BIM 360.
        if self.acc_probe_failed.load(Ordering::Relaxed) {
            if let Some(ref account_id) = self.account_id {
                return self
                    .add_user_bim360(account_id, project_id, request)
                    .await;
            }
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

        // On 400/404/500 try BIM 360 HQ v2 if we have an account_id.
        // 500 occurs when ACC endpoint is called against a BIM 360 project
        // (e.g. "reqBodyProducts is not iterable" or other server-side errors).
        if (status == 400 || status == 404 || status == 500)
            && let Some(ref account_id) = self.account_id
        {
            // Remember that ACC doesn't work for this account so subsequent
            // calls skip the probe entirely (saves ~200ms per request).
            if status == 500 {
                self.acc_probe_failed.store(true, Ordering::Relaxed);
            }
            return self
                .add_user_bim360(account_id, project_id, request)
                .await;
        }

        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to add user to project (HTTP {status}): {error_text}");
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
        let url = format!("{}/users/import", self.project_url_bim360(account_id, project_id));
        let entry = Bim360ImportEntry::from_request(&request);
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

        let result: Bim360ImportResponse = serde_json::from_str(&body_text)
            .context("Failed to parse BIM 360 import response")?;

        if result.success > 0 {
            let item = result.success_items.into_iter().next().unwrap_or(Bim360ImportItem {
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
        let error_msg = result.failure_items.into_iter()
            .flat_map(|fi| {
                fi.errors.into_iter().filter_map(|e| e.message)
            })
            .collect::<Vec<_>>()
            .join("; ");

        anyhow::bail!(
            "BIM 360 failed to add user '{}' to project: {}",
            request.email,
            if error_msg.is_empty() { "unknown error".to_string() } else { error_msg }
        )
    }

    /// Update a user's role or product access in a project
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
        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update project user ({status}): {error_text}");
        }

        let user: ProjectUser = response
            .json()
            .await
            .context("Failed to parse update user response")?;

        Ok(user)
    }

    /// Remove a user from a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `user_id` - The user ID to remove
    pub async fn remove_user(&self, project_id: &str, user_id: &str) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to remove user from project ({status}): {error_text}");
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
        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users/{}", self.project_url(project_id), user_id);

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
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/users?filter[email]={}&limit=1",
            self.project_url(project_id),
            urlencoding::encode(email),
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to find project user by email ({status}): {body}");
        }

        let page: PaginatedResponse<ProjectUser> = response
            .json()
            .await
            .context("Failed to parse project users response")?;

        Ok(page.results.into_iter().next())
    }

    /// Fetch all users in a project (handles pagination automatically)
    pub async fn list_all_project_users(&self, project_id: &str) -> Result<Vec<ProjectUser>> {
        let mut all_users = Vec::new();
        let mut offset = 0;
        let limit = 200;

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
                    suppress_administrative_emails: false,
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
            suppress_administrative_emails: false,
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
            suppress_administrative_emails: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"email\":\"user@example.com\""));
        assert!(json.contains("\"roleIds\":[\"role-456\"]"), "must send roleIds array: {json}");
        assert!(json.contains("docs"));
    }

    #[test]
    fn test_update_request_serialization() {
        let request = UpdateProjectUserRequest {
            role_ids: vec!["new-role".to_string()],
            products: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"roleIds\":[\"new-role\"]"), "must send roleIds array: {json}");
        // products should be skipped when None
        assert!(!json.contains("products"));
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
