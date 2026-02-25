// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin company and template operations

use anyhow::{Context, Result};

use raps_kernel::http;

use crate::types::{AccountProject, Company, PaginatedResponse, ProjectClassification};

use super::{AccountAdminClient, normalize_account_id};

impl AccountAdminClient {
    /// List all companies in an account
    ///
    /// Uses the HQ v1 API endpoint for companies.
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    ///
    /// # Returns
    /// A vector of all companies in the account
    pub async fn list_companies(&self, account_id: &str) -> Result<Vec<Company>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/companies", self.hq_url(&account_id));

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list companies ({status}): {error_text}");
        }

        let companies: Vec<Company> = response
            .json()
            .await
            .context("Failed to parse companies response")?;

        Ok(companies)
    }

    /// List project templates in an account (paginated)
    ///
    /// Templates are projects with classification="template".
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `limit` - Maximum results per page (max: 200)
    /// * `offset` - Starting index
    pub async fn list_templates(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountProject>> {
        self.list_projects_filtered(
            account_id,
            Some(ProjectClassification::Template),
            None,
            limit,
            offset,
        )
        .await
    }

    /// Fetch all templates in an account (handles pagination automatically)
    pub async fn list_all_templates(&self, account_id: &str) -> Result<Vec<AccountProject>> {
        let mut all_templates = Vec::new();
        let mut offset = 0;
        let limit = 200;

        loop {
            let response = self
                .list_templates(account_id, Some(limit), Some(offset))
                .await?;
            let has_more = response.has_more();
            let next_offset = response.next_offset();
            all_templates.extend(response.results);

            if !has_more {
                break;
            }
            offset = next_offset;
        }

        Ok(all_templates)
    }
}
