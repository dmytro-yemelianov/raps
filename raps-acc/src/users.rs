// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project Users API client for ACC/BIM 360

use anyhow::{Context, Result};
use serde::Serialize;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::types::{PaginatedResponse, ProductAccess, ProjectUser};

/// Client for ACC Project Users API
///
/// Provides operations for managing users within individual projects.
pub struct ProjectUsersClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

/// Request to add a user to a project
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddProjectUserRequest {
    /// User ID (Autodesk user identifier)
    pub user_id: String,
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
        let project_id = normalize_project_id(project_id);
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

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list project users")?;

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

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get project user")?;

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

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to add user to project")?;

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

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to update project user")?;

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

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to remove user from project")?;

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

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to check user existence")?;

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

    /// Import multiple users to a project at once
    ///
    /// Attempts to add each user individually and collects results.
    /// This method does not use a bulk API endpoint (which doesn't exist for project users),
    /// but instead calls add_user for each user and aggregates the results.
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
        let mut imported = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        let mut successes = Vec::new();

        for user in users {
            let email = user.email.clone();

            // Build the add user request
            // Note: We need the user_id, not email, for the API.
            // The import_users tool in MCP will need to look up user IDs by email first.
            // For now, we'll attempt to use email as user_id (the caller should resolve this)
            let request = AddProjectUserRequest {
                user_id: user.email.clone(), // Caller should provide actual user ID
                role_id: user.role_id,
                products: user.products.unwrap_or_default(),
            };

            match self.add_user(project_id, request).await {
                Ok(project_user) => {
                    imported += 1;
                    successes.push(ImportUserSuccess {
                        email,
                        user_id: Some(project_user.id),
                    });
                }
                Err(e) => {
                    failed += 1;
                    errors.push(ImportUserError {
                        email,
                        error: e.to_string(),
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

/// Remove "b." prefix from project ID if present
fn normalize_project_id(project_id: &str) -> String {
    project_id
        .strip_prefix("b.")
        .unwrap_or(project_id)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_request_serialization() {
        let request = AddProjectUserRequest {
            user_id: "user-123".to_string(),
            role_id: Some("role-456".to_string()),
            products: vec![ProductAccess {
                key: "docs".to_string(),
                access: "member".to_string(),
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("user-123"));
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
}
