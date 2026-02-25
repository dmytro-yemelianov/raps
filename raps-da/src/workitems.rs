// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! WorkItem operations for the Design Automation API.

use anyhow::{Context, Result};

use raps_kernel::http;

use crate::types::*;
use crate::DesignAutomationClient;

impl DesignAutomationClient {
    /// Create a work item (run an activity)
    pub async fn create_workitem(
        &self,
        activity_id: &str,
        arguments: std::collections::HashMap<String, WorkItemArgument>,
    ) -> Result<WorkItem> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/workitems", self.config.da_url());

        let request = CreateWorkItemRequest {
            activity_id: activity_id.to_string(),
            arguments,
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
            anyhow::bail!("Failed to create workitem ({status}): {error_text}");
        }

        let workitem: WorkItem = response
            .json()
            .await
            .context("Failed to parse workitem response")?;

        Ok(workitem)
    }

    /// List all workitems
    ///
    /// The DA API requires a `startAfterTime` query parameter.
    /// Defaults to 24 hours ago if not specified.
    pub async fn list_workitems(&self) -> Result<Vec<WorkItem>> {
        let token = self.auth.get_token().await?;
        // DA API requires startAfterTime -- default to 24h ago
        let start_after = chrono::Utc::now() - chrono::Duration::hours(24);
        let url = format!("{}/workitems", self.config.da_url());
        // DA v3 API expects Unix epoch seconds, not ISO 8601
        let start_after_str = start_after.timestamp().to_string();

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .get(&url)
                .bearer_auth(&token)
                .query(&[("startAfterTime", &start_after_str)])
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list workitems ({status}): {error_text}");
        }

        let paginated: PaginatedResponse<WorkItem> = response
            .json()
            .await
            .context("Failed to parse workitems response")?;

        Ok(paginated.data)
    }

    /// Get work item status
    pub async fn get_workitem_status(&self, id: &str) -> Result<WorkItem> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/workitems/{}", self.config.da_url(), id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get workitem status ({status}): {error_text}");
        }

        let workitem: WorkItem = response
            .json()
            .await
            .context("Failed to parse workitem response")?;

        Ok(workitem)
    }
}
