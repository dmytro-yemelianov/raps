// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Checklist operations for the ACC Extended API.

use anyhow::{Context, Result};
use serde::Deserialize;

use raps_kernel::http;

use super::AccClient;
use super::types::*;

impl AccClient {
    /// List checklists in a project
    pub async fn list_checklists(&self, project_id: &str) -> Result<Vec<Checklist>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists",
            self.config.checklists_url(),
            project_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list checklists ({status}): {error_text}");
        }

        let checklists_response: ChecklistsResponse = response
            .json()
            .await
            .context("Failed to parse checklists response")?;

        Ok(checklists_response.results)
    }

    /// List checklist templates in a project
    pub async fn list_checklist_templates(
        &self,
        project_id: &str,
    ) -> Result<Vec<ChecklistTemplate>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/templates",
            self.config.checklists_url(),
            project_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to list checklist templates ({}): {}",
                status,
                error_text
            );
        }

        #[derive(Deserialize)]
        struct TemplatesResponse {
            results: Vec<ChecklistTemplate>,
        }

        let templates_response: TemplatesResponse = response
            .json()
            .await
            .context("Failed to parse checklist templates response")?;

        Ok(templates_response.results)
    }

    /// Get a specific checklist by ID
    pub async fn get_checklist(&self, project_id: &str, checklist_id: &str) -> Result<Checklist> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists/{}",
            self.config.checklists_url(),
            project_id,
            checklist_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get checklist ({status}): {error_text}");
        }

        let checklist: Checklist = response
            .json()
            .await
            .context("Failed to parse checklist response")?;
        Ok(checklist)
    }

    /// Create a new checklist
    pub async fn create_checklist(
        &self,
        project_id: &str,
        request: CreateChecklistRequest,
    ) -> Result<Checklist> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists",
            self.config.checklists_url(),
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
            anyhow::bail!("Failed to create checklist ({status}): {error_text}");
        }

        let checklist: Checklist = response
            .json()
            .await
            .context("Failed to parse checklist response")?;
        Ok(checklist)
    }

    /// Update an existing checklist
    pub async fn update_checklist(
        &self,
        project_id: &str,
        checklist_id: &str,
        request: UpdateChecklistRequest,
    ) -> Result<Checklist> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists/{}",
            self.config.checklists_url(),
            project_id,
            checklist_id
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
            anyhow::bail!("Failed to update checklist ({status}): {error_text}");
        }

        let checklist: Checklist = response
            .json()
            .await
            .context("Failed to parse checklist response")?;
        Ok(checklist)
    }
}
