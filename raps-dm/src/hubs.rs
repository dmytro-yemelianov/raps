// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Hub, project, and top-folder operations for the Data Management API.

use anyhow::{Context, Result};

use crate::types::*;
use crate::{DataManagementClient, MAX_PAGINATION_PAGES};

impl DataManagementClient {
    // ── AEC Data Model GraphQL methods ──

    /// Execute a GraphQL query against the AEC Data Model API.
    async fn gql_query<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
    ) -> Result<T> {
        let token = self.auth.get_3leg_token().await?;
        let url = self.config.aec_graphql_url();

        tracing::info!(method = "POST", url = %raps_kernel::logging::redact_secrets(&url), "GraphQL request");

        let mut body = serde_json::json!({ "query": query });
        if let Some(vars) = variables {
            body["variables"] = vars;
        }

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&body)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "GraphQL response");

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("AEC GraphQL request failed ({status}): {error_text}");
        }

        let gql: GqlResponse<T> = response
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        if let Some(errors) = gql.errors {
            let msgs: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
            anyhow::bail!("AEC GraphQL errors: {}", msgs.join("; "));
        }

        gql.data.context("GraphQL response contained no data")
    }

    /// List hubs via the AEC Data Model GraphQL API (faster than REST).
    ///
    /// Returns the same `Hub` type as `list_hubs()` for API compatibility.
    /// Automatically paginates through all results using cursor-based pagination.
    pub async fn list_hubs_graphql(&self) -> Result<Vec<Hub>> {
        const QUERY: &str = r#"
            query GetHubs($cursor: String) {
                hubs(pagination: {cursor: $cursor, limit: 200}) {
                    results { id name }
                    pagination { cursor }
                }
            }
        "#;

        let mut all_hubs = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let vars = cursor.as_ref().map(|c| serde_json::json!({ "cursor": c }));

            let data: GqlHubsData = self.gql_query(QUERY, vars).await?;

            for gh in &data.hubs.results {
                all_hubs.push(Hub {
                    hub_type: "hubs".to_string(),
                    id: gh.id.clone(),
                    attributes: HubAttributes {
                        name: gh.name.clone(),
                        region: None,
                        extension: None,
                    },
                });
            }

            match data.hubs.pagination.and_then(|p| p.cursor) {
                Some(c) if !data.hubs.results.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        Ok(all_hubs)
    }

    /// List projects in a hub via the AEC Data Model GraphQL API (faster than REST).
    ///
    /// Returns the same `Project` type as `list_projects()` for API compatibility.
    /// The `hub_id` should be the AEC Data Model hub ID (from `list_hubs_graphql()`).
    /// Each project includes `alternativeIdentifiers.dataManagementAPIProjectId`
    /// which maps to the REST API project ID (b.xxx format).
    pub async fn list_projects_graphql(&self, hub_id: &str) -> Result<Vec<Project>> {
        const QUERY: &str = r#"
            query GetProjects($hubId: ID!, $cursor: String) {
                projects(hubId: $hubId, pagination: {cursor: $cursor, limit: 200}) {
                    results {
                        id
                        name
                        alternativeIdentifiers { dataManagementAPIProjectId }
                    }
                    pagination { cursor }
                }
            }
        "#;

        let mut all_projects = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut vars = serde_json::json!({ "hubId": hub_id });
            if let Some(c) = &cursor {
                vars["cursor"] = serde_json::json!(c);
            }

            let data: GqlProjectsData = self.gql_query(QUERY, Some(vars)).await?;

            for gp in &data.projects.results {
                // Use the DM API project ID if available, otherwise fall back to GraphQL ID
                let project_id = gp
                    .alternative_identifiers
                    .as_ref()
                    .and_then(|ai| ai.data_management_api_project_id.clone())
                    .unwrap_or_else(|| gp.id.clone());

                all_projects.push(Project {
                    project_type: "projects".to_string(),
                    id: project_id,
                    attributes: ProjectAttributes {
                        name: gp.name.clone(),
                        scopes: None,
                    },
                });
            }

            match data.projects.pagination.and_then(|p| p.cursor) {
                Some(c) if !data.projects.results.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        Ok(all_projects)
    }

    // ── REST API methods ──

    /// List all accessible hubs (requires 3-legged auth)
    pub async fn list_hubs(&self) -> Result<Vec<Hub>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs", self.config.project_url());

        // Log request in verbose/debug mode
        tracing::info!(method = "GET", url = %raps_kernel::logging::redact_secrets(&url), "HTTP request");

        // Use retry logic for API requests
        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        // Log response in verbose/debug mode
        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list hubs ({status}): {error_text}");
        }

        let api_response: JsonApiResponse<Vec<Hub>> = response
            .json()
            .await
            .context("Failed to parse hubs response")?;

        Ok(api_response.data)
    }

    /// Get hub details
    pub async fn get_hub(&self, hub_id: &str) -> Result<Hub> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs/{}", self.config.project_url(), hub_id);

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get hub ({status}): {error_text}");
        }

        let api_response: JsonApiResponse<Hub> = response
            .json()
            .await
            .context("Failed to parse hub response")?;

        Ok(api_response.data)
    }

    /// List projects in a hub
    ///
    /// Follows pagination links to return complete result set (max 100 pages).
    pub async fn list_projects(&self, hub_id: &str) -> Result<Vec<Project>> {
        let token = self.auth.get_3leg_token().await?;
        let mut next_url = Some(format!(
            "{}/hubs/{}/projects",
            self.config.project_url(),
            hub_id
        ));
        let mut all_items = Vec::new();

        for _page in 0..MAX_PAGINATION_PAGES {
            let url = match next_url.take() {
                Some(u) => u,
                None => break,
            };

            let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
                self.http_client.get(&url).bearer_auth(&token)
            })
            .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to list projects ({status}): {error_text}");
            }

            let api_response: JsonApiResponse<Vec<Project>> = response
                .json()
                .await
                .context("Failed to parse projects response")?;

            all_items.extend(api_response.data);

            next_url = api_response
                .links
                .and_then(|l| l.next)
                .map(|link| link.href().to_string());
        }

        Ok(all_items)
    }

    /// Get project details
    pub async fn get_project(&self, hub_id: &str, project_id: &str) -> Result<Project> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/hubs/{}/projects/{}",
            self.config.project_url(),
            hub_id,
            project_id
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project ({status}): {error_text}");
        }

        let api_response: JsonApiResponse<Project> = response
            .json()
            .await
            .context("Failed to parse project response")?;

        Ok(api_response.data)
    }

    /// Get the issues container ID for a project.
    ///
    /// BIM360 projects store issues in a container whose ID differs from the project ID.
    /// ACC projects may also expose this. The container ID is found in
    /// `project.relationships.issues.data.id` (type = `issueContainerId`).
    /// Returns `None` if the project has no issues relationship.
    pub async fn get_issues_container_id(
        &self,
        hub_id: &str,
        project_id: &str,
    ) -> Result<Option<String>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/hubs/{}/projects/{}",
            self.config.project_url(),
            hub_id,
            project_id
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project ({status}): {error_text}");
        }

        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse project response")?;

        // Navigate: data.relationships.issues.data.id
        let container_id = body
            .pointer("/data/relationships/issues/data/id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(container_id)
    }

    /// Get top folders for a project
    pub async fn get_top_folders(&self, hub_id: &str, project_id: &str) -> Result<Vec<Folder>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/hubs/{}/projects/{}/topFolders",
            self.config.project_url(),
            hub_id,
            project_id
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get top folders ({status}): {error_text}");
        }

        let api_response: JsonApiResponse<Vec<Folder>> = response
            .json()
            .await
            .context("Failed to parse folders response")?;

        Ok(api_response.data)
    }
}
