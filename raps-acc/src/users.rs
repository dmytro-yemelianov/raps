// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project Users API client for ACC/BIM 360

use std::sync::Arc;

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
#[derive(Clone)]
pub struct ProjectUsersClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

/// Request to add a user to a project
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectUserRequest {
    /// User email address
    pub email: String,
    /// Role ID to assign (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    /// Product access configurations
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub products: Vec<ProductAccess>,
}

/// Request to update a project user
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectUserRequest {
    /// New role ID to assign
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
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
    /// Optional role ID to assign
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
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

impl ProjectUsersClient {
    /// Create a new Project Users client
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

    /// Get the base URL for Project Admin API
    fn project_url(&self, project_id: &str) -> String {
        let project_id = crate::strip_project_prefix(project_id);
        format!(
            "{}/construction/admin/v1/projects/{}",
            self.config.base_url, project_id
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

    /// Add a user to a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `request` - Add user request with user ID, role, and products
    ///
    /// # Returns
    /// The newly created project user membership
    pub async fn add_user(
        &self,
        project_id: &str,
        request: AddProjectUserRequest,
    ) -> Result<ProjectUser> {
        let token = self.auth.get_3leg_token().await?;

        let url = format!("{}/users", self.project_url(project_id));

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to add user to project ({status}): {error_text}");
        }

        let user: ProjectUser = response
            .json()
            .await
            .context("Failed to parse add user response")?;

        Ok(user)
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
                    role_id: user.role_id,
                    products: user.products.unwrap_or_default(),
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
    fn test_add_request_role_id_absent_when_none() {
        let request = AddProjectUserRequest {
            email: "user@example.com".to_string(),
            role_id: None,
            products: vec![],
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"email\":\"user@example.com\""));
        assert!(
            !json.contains("roleId"),
            "roleId must be absent when None (skip_serializing_if)"
        );
    }

    #[test]
    fn test_add_request_serialization() {
        let request = AddProjectUserRequest {
            email: "user@example.com".to_string(),
            role_id: Some("role-456".to_string()),
            products: vec![ProductAccess {
                key: "docs".to_string(),
                access: "member".to_string(),
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"email\":\"user@example.com\""));
        assert!(json.contains("role-456"));
        assert!(json.contains("docs"));
    }

    #[test]
    fn test_update_request_serialization() {
        let request = UpdateProjectUserRequest {
            role_id: Some("new-role".to_string()),
            products: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("new-role"));
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
