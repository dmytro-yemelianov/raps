// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Submittal operations for the ACC Extended API.

use anyhow::{Context, Result};

use raps_kernel::http;

use super::types::*;
use super::AccClient;

impl AccClient {
    /// List submittals in a project
    pub async fn list_submittals(&self, project_id: &str) -> Result<Vec<Submittal>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items",
            self.config.submittals_url(),
            project_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list submittals ({status}): {error_text}");
        }

        let submittals_response: SubmittalsResponse = response
            .json()
            .await
            .context("Failed to parse submittals response")?;

        Ok(submittals_response.results)
    }

    /// Get a specific submittal by ID
    pub async fn get_submittal(&self, project_id: &str, submittal_id: &str) -> Result<Submittal> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.submittals_url(),
            project_id,
            submittal_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get submittal ({status}): {error_text}");
        }

        let submittal: Submittal = response
            .json()
            .await
            .context("Failed to parse submittal response")?;
        Ok(submittal)
    }

    /// Create a new submittal
    pub async fn create_submittal(
        &self,
        project_id: &str,
        request: CreateSubmittalRequest,
    ) -> Result<Submittal> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items",
            self.config.submittals_url(),
            project_id
        );

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
            anyhow::bail!("Failed to create submittal ({status}): {error_text}");
        }

        let submittal: Submittal = response
            .json()
            .await
            .context("Failed to parse submittal response")?;
        Ok(submittal)
    }

    /// Update an existing submittal
    pub async fn update_submittal(
        &self,
        project_id: &str,
        submittal_id: &str,
        request: UpdateSubmittalRequest,
    ) -> Result<Submittal> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.submittals_url(),
            project_id,
            submittal_id
        );

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
            anyhow::bail!("Failed to update submittal ({status}): {error_text}");
        }

        let submittal: Submittal = response
            .json()
            .await
            .context("Failed to parse submittal response")?;
        Ok(submittal)
    }

    /// Delete a submittal
    pub async fn delete_submittal(&self, project_id: &str, submittal_id: &str) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.submittals_url(),
            project_id,
            submittal_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete submittal ({status}): {error_text}");
        }

        Ok(())
    }
}
