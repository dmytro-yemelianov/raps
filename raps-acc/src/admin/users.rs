// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin user operations

use anyhow::{Context, Result};

use raps_kernel::http;

use crate::types::{AccountUser, PaginatedResponse};

use super::types::UpdateAccountUserRequest;
use super::{AccountAdminClient, normalize_account_id};

impl AccountAdminClient {
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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request_body)
        })
        .await?;

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

    /// Update an account user's properties (company, status, etc.)
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `user_id` - The user ID to update
    /// * `request` - Update parameters
    pub async fn update_user(
        &self,
        account_id: &str,
        user_id: &str,
        request: UpdateAccountUserRequest,
    ) -> Result<AccountUser> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/users/{}", self.admin_url(&account_id), user_id);

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
            anyhow::bail!("Failed to update account user ({status}): {error_text}");
        }

        let user: AccountUser = response
            .json()
            .await
            .context("Failed to parse user update response")?;

        Ok(user)
    }
}
