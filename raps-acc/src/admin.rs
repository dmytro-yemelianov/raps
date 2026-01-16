// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin API client for ACC/BIM 360

use anyhow::{Context, Result};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::types::{AccountProject, AccountUser, PaginatedResponse};

/// Client for ACC Account Admin API
///
/// Provides operations for managing users and projects at the account level.
/// Requires account admin privileges.
pub struct AccountAdminClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
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

    /// List all users in an account (paginated)
    ///
    /// # Arguments
    /// * `account_id` - The account ID (without "b." prefix if present)
    /// * `limit` - Maximum number of results per page (max: 200)
    /// * `offset` - Starting index for pagination
    pub async fn list_users(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountUser>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let mut url = format!("{}/users", self.admin_url(&account_id));

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
            .context("Failed to list account users")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list users ({status}): {error_text}");
        }

        let users_response: PaginatedResponse<AccountUser> = response
            .json()
            .await
            .context("Failed to parse users response")?;

        Ok(users_response)
    }

    /// Search for a user by email address
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `email` - Email address to search for
    ///
    /// # Returns
    /// The user if found, None if not found
    pub async fn find_user_by_email(
        &self,
        account_id: &str,
        email: &str,
    ) -> Result<Option<AccountUser>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/users/search", self.admin_url(&account_id));

        let request_body = serde_json::json!({
            "email": email
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to search for user")?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to search for user ({status}): {error_text}");
        }

        // The search endpoint returns a single user or array
        let user: AccountUser = response
            .json()
            .await
            .context("Failed to parse user search response")?;

        Ok(Some(user))
    }

    /// List all projects in an account (paginated)
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `limit` - Maximum results per page (max: 200)
    /// * `offset` - Starting index
    pub async fn list_projects(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountProject>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let mut url = format!("{}/projects", self.admin_url(&account_id));

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
            .context("Failed to list projects")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list projects ({status}): {error_text}");
        }

        let projects_response: PaginatedResponse<AccountProject> = response
            .json()
            .await
            .context("Failed to parse projects response")?;

        Ok(projects_response)
    }

    /// Get details of a specific project
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `project_id` - The project ID
    pub async fn get_project(&self, account_id: &str, project_id: &str) -> Result<AccountProject> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);
        let project_id = normalize_project_id(project_id);

        let url = format!("{}/projects/{}", self.admin_url(&account_id), project_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get project")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project ({status}): {error_text}");
        }

        let project: AccountProject = response
            .json()
            .await
            .context("Failed to parse project response")?;

        Ok(project)
    }

    /// Fetch all users in an account (handles pagination automatically)
    ///
    /// This is a convenience method that iterates through all pages.
    /// Use with caution for accounts with many users.
    pub async fn list_all_users(&self, account_id: &str) -> Result<Vec<AccountUser>> {
        let mut all_users = Vec::new();
        let mut offset = 0;
        let limit = 200; // Maximum allowed

        loop {
            let response = self
                .list_users(account_id, Some(limit), Some(offset))
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

    /// Fetch all projects in an account (handles pagination automatically)
    ///
    /// This is a convenience method that iterates through all pages.
    pub async fn list_all_projects(&self, account_id: &str) -> Result<Vec<AccountProject>> {
        let mut all_projects = Vec::new();
        let mut offset = 0;
        let limit = 200; // Maximum allowed

        loop {
            let response = self
                .list_projects(account_id, Some(limit), Some(offset))
                .await?;
            let has_more = response.has_more();
            let next_offset = response.next_offset();
            all_projects.extend(response.results);

            if !has_more {
                break;
            }
            offset = next_offset;
        }

        Ok(all_projects)
    }
}

/// Remove "b." prefix from account ID if present
fn normalize_account_id(account_id: &str) -> String {
    account_id
        .strip_prefix("b.")
        .unwrap_or(account_id)
        .to_string()
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
    fn test_normalize_account_id() {
        assert_eq!(normalize_account_id("b.123-456"), "123-456");
        assert_eq!(normalize_account_id("123-456"), "123-456");
    }

    #[test]
    fn test_normalize_project_id() {
        assert_eq!(normalize_project_id("b.proj-123"), "proj-123");
        assert_eq!(normalize_project_id("proj-123"), "proj-123");
    }
}
