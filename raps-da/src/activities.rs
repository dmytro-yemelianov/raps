// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Activity operations for the Design Automation API.

use anyhow::{Context, Result};
use serde::Serialize;

use raps_kernel::http;

use crate::types::*;
use crate::DesignAutomationClient;

impl DesignAutomationClient {
    /// List all activities
    pub async fn list_activities(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/activities", self.config.da_url());

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list activities ({status}): {error_text}");
        }

        let paginated: PaginatedResponse<String> = response
            .json()
            .await
            .context("Failed to parse activities response")?;

        Ok(paginated.data)
    }

    /// Create a new activity
    pub async fn create_activity(&self, request: CreateActivityRequest) -> Result<Activity> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/activities", self.config.da_url());

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
            anyhow::bail!("Failed to create activity ({status}): {error_text}");
        }

        let activity: Activity = response
            .json()
            .await
            .context("Failed to parse activity response")?;

        Ok(activity)
    }

    /// Create an alias for an activity
    pub async fn create_activity_alias(
        &self,
        activity_id: &str,
        alias: &str,
        version: i32,
    ) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/activities/{}/aliases",
            self.config.da_url(),
            activity_id
        );

        #[derive(Serialize)]
        struct AliasRequest {
            id: String,
            version: i32,
        }

        let request = AliasRequest {
            id: alias.to_string(),
            version,
        };

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
            anyhow::bail!("Failed to create activity alias ({status}): {error_text}");
        }

        Ok(())
    }

    /// Delete an activity
    pub async fn delete_activity(&self, id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/activities/{}", self.config.da_url(), id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete activity ({status}): {error_text}");
        }

        Ok(())
    }
}
