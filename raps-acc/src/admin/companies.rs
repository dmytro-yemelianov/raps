// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin company and template operations

use anyhow::{Context, Result};

use raps_kernel::http;

use crate::types::{AccountProject, Company, PaginatedResponse, ProjectClassification};

use super::types::{CreateCompanyRequest, UpdateCompanyRequest};
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
        let token = self.auth.get_token().await?;
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

    /// Create a new company in an account
    ///
    /// Uses the HQ v1 API endpoint POST /hq/v1/accounts/:account_id/companies.
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `request` - The company creation request
    ///
    /// # Returns
    /// The created company
    pub async fn create_company(
        &self,
        account_id: &str,
        request: CreateCompanyRequest,
    ) -> Result<Company> {
        let token = self.auth.get_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/companies", self.hq_url(&account_id));

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create company ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse create company response")
    }

    /// Update an existing company in an account
    ///
    /// Uses the HQ v1 API endpoint PATCH /hq/v1/accounts/:account_id/companies/:company_id.
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `company_id` - The company ID to update
    /// * `request` - The company update request
    ///
    /// # Returns
    /// The updated company
    pub async fn update_company(
        &self,
        account_id: &str,
        company_id: &str,
        request: UpdateCompanyRequest,
    ) -> Result<Company> {
        let token = self.auth.get_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/companies/{}", self.hq_url(&account_id), company_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update company ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse update company response")
    }

    /// Get a single company by ID
    ///
    /// Uses the HQ v1 API endpoint GET /hq/v1/accounts/:account_id/companies/:company_id.
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `company_id` - The company ID to retrieve
    ///
    /// # Returns
    /// The company details
    pub async fn get_company(&self, account_id: &str, company_id: &str) -> Result<Company> {
        let token = self.auth.get_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/companies/{}", self.hq_url(&account_id), company_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get company ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse get company response")
    }

    /// Search companies in an account by name
    ///
    /// Uses the HQ v1 API endpoint GET /hq/v1/accounts/:account_id/companies/search?name=X.
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `name` - The company name search term
    ///
    /// # Returns
    /// A vector of matching companies
    pub async fn search_companies(&self, account_id: &str, name: &str) -> Result<Vec<Company>> {
        let token = self.auth.get_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!(
            "{}/companies/search?name={}",
            self.hq_url(&account_id),
            urlencoding::encode(name)
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to search companies ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse search companies response")
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
        let limit = 100;

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
