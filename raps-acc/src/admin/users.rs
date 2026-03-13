// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin user operations

use anyhow::{Context, Result};

use raps_kernel::error::RapsError;
use raps_kernel::http;

use crate::types::{AccountUser, PaginatedResponse, PaginationInfo};

use super::types::{CreateAccountUserRequest, UpdateAccountUserRequest};
use super::{AccountAdminClient, normalize_account_id};

impl AccountAdminClient {
    /// Build query string for user list pagination
    fn build_user_query_params(limit: Option<usize>, offset: Option<usize>) -> String {
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l.min(100)));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        }
    }

    /// List all users in an account (paginated)
    ///
    /// Tries ACC Construction Admin v1 first. Falls back to BIM 360 HQ v1
    /// if the account is a BIM 360 Business hub (HTTP 400/404).
    ///
    /// # Arguments
    /// * `account_id` - The account ID (without "b." prefix if present)
    /// * `limit` - Maximum number of results per page (max: 100)
    /// * `offset` - Starting index for pagination
    pub async fn list_users(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountUser>> {
        let account_id = normalize_account_id(account_id);
        let query = Self::build_user_query_params(limit, offset);

        // Try ACC v1 first (3-legged) — skip if not logged in
        if let Ok(token) = self.auth.get_3leg_token().await {
            let url = format!("{}/users{}", self.admin_url(&account_id), query);

            let response = http::send_with_retry(&self.config.http_config, || {
                self.http_client.get(&url).bearer_auth(&token)
            })
            .await?;

            if response.status().is_success() {
                let users_response: PaginatedResponse<AccountUser> = response
                    .json()
                    .await
                    .context("Failed to parse users response")?;
                return Ok(users_response);
            }

            let status = response.status().as_u16();
            if status != 400 && status != 404 {
                return Err(RapsError::from_response(response).await.into());
            }
        }

        // Fall back to BIM 360 HQ v1 (2-legged auth)
        self.list_users_bim360(&account_id, limit, offset).await
    }

    /// List users via BIM 360 HQ v1 API (for Business hubs).
    ///
    /// Uses 2-legged auth as required by the HQ v1 endpoint.
    /// Endpoint: GET /hq/v1/accounts/:account_id/users
    ///
    /// BIM 360 HQ v1 returns a plain JSON array (not a paginated wrapper),
    /// so we parse it and construct a PaginatedResponse manually.
    async fn list_users_bim360(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountUser>> {
        let token = self.auth.get_token().await?;
        let query = Self::build_user_query_params(limit, offset);

        let url = format!("{}/users{}", self.hq_url(account_id), query);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        // BIM 360 HQ v1 returns a plain JSON array, not {"results":[], "pagination":{}}
        let users: Vec<AccountUser> = response
            .json()
            .await
            .context("Failed to parse BIM 360 users response")?;

        let page_limit = limit.unwrap_or(100).min(100);
        let page_offset = offset.unwrap_or(0);
        let count = users.len();

        Ok(PaginatedResponse {
            results: users,
            pagination: PaginationInfo {
                limit: page_limit,
                offset: page_offset,
                // HQ v1 doesn't return total count in the body; estimate from page size
                total_results: if count < page_limit {
                    page_offset + count
                } else {
                    // May have more pages — signal that by setting total > offset + count
                    page_offset + count + 1
                },
            },
        })
    }

    /// Search for a user by email address
    ///
    /// Tries ACC Construction Admin v1 first. Falls back to BIM 360 HQ v1
    /// if the account is a BIM 360 Business hub (HTTP 400/404).
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
        let account_id = normalize_account_id(account_id);

        // Try ACC v1 first (POST /users/search with JSON body) — skip if not logged in
        if let Ok(token) = self.auth.get_3leg_token().await {
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

            let status_code = response.status().as_u16();

            if response.status().is_success() {
                let user: AccountUser = response
                    .json()
                    .await
                    .context("Failed to parse user search response")?;
                return Ok(Some(user));
            }

            if status_code != 400 && status_code != 404 {
                return Err(RapsError::from_response(response).await.into());
            }
        }

        // Fall back to BIM 360 HQ v1 (GET /users/search?email=... with 2-legged auth)
        let token_2leg = self.auth.get_token().await?;
        let url = format!(
            "{}/users/search?email={}",
            self.hq_url(&account_id),
            urlencoding::encode(email)
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token_2leg)
        })
        .await?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        // BIM 360 HQ v1 /users/search may return either:
        // - A plain user object (matching AccountUser)
        // - An array of user objects
        // - A wrapper with different field names (snake_case)
        // Parse as raw JSON first, then extract fields flexibly.
        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse BIM 360 user search response")?;

        // Try direct deserialization first
        if let Ok(user) = serde_json::from_value::<AccountUser>(body.clone()) {
            return Ok(Some(user));
        }

        // If the response is an array, take the first matching element
        if let Some(arr) = body.as_array() {
            for item in arr {
                if let Ok(user) = serde_json::from_value::<AccountUser>(item.clone()) {
                    if user.email.to_lowercase() == email.to_lowercase() {
                        return Ok(Some(user));
                    }
                }
            }
            // Try manual extraction on first element
            if let Some(first) = arr.first() {
                if let Ok(user) = serde_json::from_value::<AccountUser>(first.clone()) {
                    return Ok(Some(user));
                }
            }
        }

        // Manual extraction for non-standard BIM 360 response shapes
        let obj = body
            .as_object()
            .ok_or_else(|| anyhow::anyhow!(
                "Unexpected BIM 360 search response format: {}",
                serde_json::to_string(&body).unwrap_or_default().chars().take(200).collect::<String>()
            ))?;
        let id = obj
            .get("id")
            .or_else(|| obj.get("uid"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let user_email = obj
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or(email)
            .to_string();
        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from);
        let first_name = obj
            .get("first_name")
            .or_else(|| obj.get("firstName"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let last_name = obj
            .get("last_name")
            .or_else(|| obj.get("lastName"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let company_id = obj
            .get("company_id")
            .or_else(|| obj.get("companyId"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(Some(AccountUser {
            id,
            email: user_email,
            name,
            first_name,
            last_name,
            company_id,
            status,
            added_on: None,
        }))
    }

    /// Fetch all users in an account (handles pagination automatically)
    ///
    /// This is a convenience method that iterates through all pages.
    /// Use with caution for accounts with many users.
    pub async fn list_all_users(&self, account_id: &str) -> Result<Vec<AccountUser>> {
        let mut all_users = Vec::new();
        let mut offset = 0;
        let limit = 100; // Maximum allowed per APS docs

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
    /// Tries ACC Construction Admin v1 first. Falls back to BIM 360 HQ v1
    /// if the account is a BIM 360 Business hub (HTTP 400/404).
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
        let account_id = normalize_account_id(account_id);

        // Try ACC v1 first (3-legged) — skip if not logged in
        if let Ok(token) = self.auth.get_3leg_token().await {
            let url = format!("{}/users/{}", self.admin_url(&account_id), user_id);

            let response = http::send_with_retry(&self.config.http_config, || {
                self.http_client
                    .patch(&url)
                    .bearer_auth(&token)
                    .header("Content-Type", "application/json")
                    .json(&request)
            })
            .await?;

            if response.status().is_success() {
                let user: AccountUser = response
                    .json()
                    .await
                    .context("Failed to parse user update response")?;
                return Ok(user);
            }

            let status = response.status().as_u16();
            if status != 400 && status != 404 {
                return Err(RapsError::from_response(response).await.into());
            }
        }

        // Fall back to BIM 360 HQ v1 (2-legged auth)
        let token_2leg = self.auth.get_token().await?;
        let url = format!("{}/users/{}", self.hq_url(&account_id), user_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token_2leg)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let user: AccountUser = response
            .json()
            .await
            .context("Failed to parse BIM 360 user update response")?;

        Ok(user)
    }

    /// Create (invite) a new user at account level.
    ///
    /// Uses HQ v1 with 2-legged auth (POST /hq/v1/accounts/:account_id/users).
    pub async fn create_user(
        &self,
        account_id: &str,
        request: CreateAccountUserRequest,
    ) -> Result<AccountUser> {
        let token = self.auth.get_token().await?;
        let account_id = normalize_account_id(account_id);
        let url = format!("{}/users", self.hq_url(&account_id));

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        response
            .json()
            .await
            .context("Failed to parse create user response")
    }

    /// Get a single account user by ID.
    ///
    /// Tries ACC v1 (3-legged) first, falls back to BIM 360 HQ v1 (2-legged).
    pub async fn get_user(&self, account_id: &str, user_id: &str) -> Result<AccountUser> {
        let account_id = normalize_account_id(account_id);

        // Try ACC v1 first (3-legged) — skip if not logged in
        if let Ok(token) = self.auth.get_3leg_token().await {
            let url = format!("{}/users/{}", self.admin_url(&account_id), user_id);

            let response = http::send_with_retry(&self.config.http_config, || {
                self.http_client.get(&url).bearer_auth(&token)
            })
            .await?;

            if response.status().is_success() {
                return response
                    .json()
                    .await
                    .context("Failed to parse user response");
            }

            let status = response.status().as_u16();
            if status != 400 && status != 404 {
                return Err(RapsError::from_response(response).await.into());
            }
        }

        // Fall back to BIM 360 HQ v1 (2-legged)
        let token_2leg = self.auth.get_token().await?;
        let url = format!("{}/users/{}", self.hq_url(&account_id), user_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token_2leg)
        })
        .await?;

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        response
            .json()
            .await
            .context("Failed to parse BIM 360 user response")
    }
}
