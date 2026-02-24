// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Design Automation API module
//!
//! Handles automation of CAD processing with engines like AutoCAD, Revit, Inventor, 3ds Max.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::{self, HttpClientConfig};

/// Engine information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    pub id: String,
    pub description: Option<String>,
    pub product_version: Option<String>,
}

/// AppBundle information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBundle {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: Option<i32>,
}

/// AppBundle details (full)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBundleDetails {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: i32,
    pub package: Option<String>,
    pub upload_parameters: Option<UploadParameters>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadParameters {
    pub endpoint_url: Option<String>,
    pub form_data: Option<std::collections::HashMap<String, String>>,
}

/// Activity information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: Option<i32>,
    pub command_line: Option<Vec<String>>,
    pub app_bundles: Option<Vec<String>>,
}

/// WorkItem information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: String,
    pub status: String,
    pub progress: Option<String>,
    pub report_url: Option<String>,
    pub stats: Option<WorkItemStats>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemStats {
    pub time_queued: Option<String>,
    pub time_download_started: Option<String>,
    pub time_instruction_started: Option<String>,
    pub time_instruction_ended: Option<String>,
    pub time_upload_ended: Option<String>,
    pub time_finished: Option<String>,
    pub bytes_downloaded: Option<i64>,
    pub bytes_uploaded: Option<i64>,
}

/// Request to create an AppBundle
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppBundleRequest {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
}

/// Request to create an Activity
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActivityRequest {
    pub id: String,
    pub engine: String,
    pub command_line: Vec<String>,
    pub app_bundles: Vec<String>,
    pub parameters: std::collections::HashMap<String, ActivityParameter>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityParameter {
    pub verb: String,
    pub local_name: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<bool>,
}

/// Request to create a WorkItem
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkItemRequest {
    pub activity_id: String,
    pub arguments: std::collections::HashMap<String, WorkItemArgument>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemArgument {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Paginated response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination_token: Option<String>,
}

/// Design Automation API client
#[derive(Clone)]
pub struct DesignAutomationClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl DesignAutomationClient {
    /// Create a new Design Automation client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Design Automation client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> Self {
        // Create HTTP client with configured timeouts
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new()); // Fallback to default if config fails

        Self {
            config,
            auth,
            http_client,
        }
    }

    /// Get the configured nickname (or "default")
    pub fn nickname(&self) -> &str {
        self.config.da_nickname.as_deref().unwrap_or("default")
    }

    /// Fetch the effective nickname from the DA API.
    ///
    /// Returns the configured nickname if set, otherwise calls
    /// `GET /forgeapps/me` to get the actual owner name (usually the client_id).
    pub async fn effective_nickname(&self) -> Result<String> {
        if let Some(ref nick) = self.config.da_nickname {
            return Ok(nick.clone());
        }
        let token = self.auth.get_token().await?;
        let url = format!("{}/forgeapps/me", self.config.da_url());
        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;
        if response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            // Response is a plain string (the nickname) wrapped in quotes
            let trimmed = text.trim().trim_matches('"');
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
        Ok("default".to_string())
    }

    /// List available engines
    ///
    /// Returns a list of engine IDs (e.g., "Autodesk.Revit+2024").
    /// Use `get_engine` to fetch full details for a specific engine.
    pub async fn list_engines(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/engines", self.config.da_url());

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list engines ({status}): {error_text}");
        }

        let paginated: PaginatedResponse<String> = response
            .json()
            .await
            .context("Failed to parse engines response")?;

        Ok(paginated.data)
    }

    /// List all engines with pagination, returning structured Engine objects.
    ///
    /// The API returns engine IDs as strings. This method parses the ID to
    /// extract product name and version as the description.
    pub async fn list_engines_detailed(&self) -> Result<Vec<Engine>> {
        let token = self.auth.get_token().await?;
        let base_url = format!("{}/engines", self.config.da_url());
        let mut all_engines = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let url = match &page_token {
                Some(tok) => format!("{base_url}?page={tok}"),
                None => base_url.clone(),
            };

            let token_clone = token.clone();
            let response = http::send_with_retry(&self.config.http_config, || {
                self.http_client.get(&url).bearer_auth(&token_clone)
            })
            .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to list engines ({status}): {error_text}");
            }

            let paginated: PaginatedResponse<String> = response
                .json()
                .await
                .context("Failed to parse engines response")?;

            // Convert string IDs to Engine structs, parsing description from the ID.
            // Format: "Autodesk.ProductName+VersionNumber"
            for id in paginated.data {
                let description = id
                    .split('.')
                    .next_back()
                    .map(|s| s.replace('+', " "))
                    .unwrap_or_default();
                all_engines.push(Engine {
                    id,
                    description: Some(description),
                    product_version: None,
                });
            }

            match paginated.pagination_token {
                Some(tok) if !tok.is_empty() => page_token = Some(tok),
                _ => break,
            }
        }

        Ok(all_engines)
    }

    /// List all app bundles
    pub async fn list_appbundles(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles", self.config.da_url());

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list appbundles ({status}): {error_text}");
        }

        let paginated: PaginatedResponse<String> = response
            .json()
            .await
            .context("Failed to parse appbundles response")?;

        Ok(paginated.data)
    }

    /// Create a new app bundle
    pub async fn create_appbundle(
        &self,
        id: &str,
        engine: &str,
        description: Option<&str>,
    ) -> Result<AppBundleDetails> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles", self.config.da_url());

        let request = CreateAppBundleRequest {
            id: id.to_string(),
            engine: engine.to_string(),
            description: description.map(|s| s.to_string()),
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
            anyhow::bail!("Failed to create appbundle ({status}): {error_text}");
        }

        let appbundle: AppBundleDetails = response
            .json()
            .await
            .context("Failed to parse appbundle response")?;

        Ok(appbundle)
    }

    /// Create an alias for an app bundle
    pub async fn create_appbundle_alias(
        &self,
        bundle_id: &str,
        alias: &str,
        version: i32,
    ) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles/{}/aliases", self.config.da_url(), bundle_id);

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
            anyhow::bail!("Failed to create appbundle alias ({status}): {error_text}");
        }

        Ok(())
    }

    /// Delete an app bundle
    pub async fn delete_appbundle(&self, id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles/{}", self.config.da_url(), id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete appbundle ({status}): {error_text}");
        }

        Ok(())
    }

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
        // DA API requires startAfterTime — default to 24h ago
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_deserialization() {
        let json = r#"{
            "id": "Autodesk.Revit+2024",
            "description": "Revit 2024 Engine",
            "productVersion": "2024"
        }"#;

        let engine: Engine = serde_json::from_str(json).unwrap();
        assert_eq!(engine.id, "Autodesk.Revit+2024");
        assert_eq!(engine.description, Some("Revit 2024 Engine".to_string()));
    }

    #[test]
    fn test_appbundle_deserialization() {
        let json = r#"{
            "id": "myapp.MyBundle+dev",
            "engine": "Autodesk.Revit+2024",
            "description": "My custom bundle",
            "version": 1
        }"#;

        let bundle: AppBundle = serde_json::from_str(json).unwrap();
        assert_eq!(bundle.id, "myapp.MyBundle+dev");
        assert_eq!(bundle.engine, "Autodesk.Revit+2024");
    }

    #[test]
    fn test_activity_deserialization() {
        let json = r#"{
            "id": "myapp.MyActivity+dev",
            "engine": "Autodesk.Revit+2024",
            "description": "My activity",
            "version": 1
        }"#;

        let activity: Activity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.id, "myapp.MyActivity+dev");
    }

    #[test]
    fn test_workitem_deserialization() {
        let json = r#"{
            "id": "workitem-id-123",
            "status": "pending",
            "progress": "0%"
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert_eq!(workitem.id, "workitem-id-123");
        assert_eq!(workitem.status, "pending");
    }

    #[test]
    fn test_workitem_stats_deserialization() {
        let json = r#"{
            "id": "workitem-id-123",
            "status": "success",
            "stats": {
                "bytesDownloaded": 1024,
                "bytesUploaded": 2048
            }
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert!(workitem.stats.is_some());
        let stats = workitem.stats.unwrap();
        assert_eq!(stats.bytes_downloaded, Some(1024));
    }

    #[test]
    fn test_create_appbundle_request_serialization() {
        let request = CreateAppBundleRequest {
            id: "MyBundle".to_string(),
            engine: "Autodesk.Revit+2024".to_string(),
            description: Some("Test bundle".to_string()),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["id"], "MyBundle");
        assert_eq!(json["engine"], "Autodesk.Revit+2024");
    }

    #[test]
    fn test_create_activity_request_serialization() {
        let mut parameters = std::collections::HashMap::new();
        parameters.insert(
            "input".to_string(),
            ActivityParameter {
                verb: "get".to_string(),
                local_name: Some("input.rvt".to_string()),
                description: None,
                required: Some(true),
                zip: None,
            },
        );

        let request = CreateActivityRequest {
            id: "MyActivity".to_string(),
            engine: "Autodesk.Revit+2024".to_string(),
            command_line: vec!["$(engine.path)\\revitcoreconsole.exe".to_string()],
            app_bundles: vec!["myapp.MyBundle+dev".to_string()],
            description: Some("Test activity".to_string()),
            parameters,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["id"], "MyActivity");
        assert!(json["commandLine"].is_array());
    }

    #[test]
    fn test_create_workitem_request_serialization() {
        let mut arguments = std::collections::HashMap::new();
        arguments.insert(
            "input".to_string(),
            WorkItemArgument {
                url: "https://example.com/input.rvt".to_string(),
                verb: Some("get".to_string()),
                headers: None,
            },
        );

        let request = CreateWorkItemRequest {
            activity_id: "myapp.MyActivity+dev".to_string(),
            arguments,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["activityId"], "myapp.MyActivity+dev");
    }

    #[test]
    fn test_paginated_response_deserialization() {
        let json = r#"{
            "paginationToken": "next-page-token",
            "data": [
                {"id": "item1", "engine": "engine1"},
                {"id": "item2", "engine": "engine2"}
            ]
        }"#;

        let response: PaginatedResponse<AppBundle> = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.pagination_token,
            Some("next-page-token".to_string())
        );
        assert_eq!(response.data.len(), 2);
    }

    #[test]
    fn test_workitem_with_progress() {
        let json = r#"{
            "id": "workitem-id",
            "status": "inprogress",
            "progress": "50%"
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert_eq!(workitem.status, "inprogress");
        assert_eq!(workitem.progress, Some("50%".to_string()));
    }

    #[test]
    fn test_workitem_with_report_url() {
        let json = r#"{
            "id": "workitem-id",
            "status": "success",
            "reportUrl": "https://example.com/report.txt"
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert!(workitem.report_url.is_some());
    }

    #[test]
    fn test_activity_parameter_serialization() {
        let param = ActivityParameter {
            verb: "get".to_string(),
            local_name: Some("input.rvt".to_string()),
            description: Some("Input file".to_string()),
            required: Some(true),
            zip: Some(false),
        };

        let json = serde_json::to_value(&param).unwrap();
        assert_eq!(json["verb"], "get");
        assert_eq!(json["localName"], "input.rvt");
        assert_eq!(json["required"], true);
    }

    #[test]
    fn test_workitem_argument_with_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        let arg = WorkItemArgument {
            url: "https://example.com/file.rvt".to_string(),
            verb: Some("get".to_string()),
            headers: Some(headers),
        };

        let json = serde_json::to_value(&arg).unwrap();
        assert_eq!(json["url"], "https://example.com/file.rvt");
        assert_eq!(json["headers"]["Authorization"], "Bearer token");
    }

    #[test]
    fn test_engine_with_product_version() {
        let json = r#"{
            "id": "Autodesk.Revit+2024",
            "productVersion": "2024"
        }"#;

        let engine: Engine = serde_json::from_str(json).unwrap();
        assert_eq!(engine.id, "Autodesk.Revit+2024");
        assert_eq!(engine.product_version, Some("2024".to_string()));
    }

    #[test]
    fn test_paginated_workitem_response_deserialization() {
        let json = r#"{
            "paginationToken": "next-token-abc",
            "data": [
                {
                    "id": "wi-001",
                    "status": "success",
                    "progress": "100%",
                    "reportUrl": "https://example.com/report1.txt"
                },
                {
                    "id": "wi-002",
                    "status": "pending"
                }
            ]
        }"#;

        let response: PaginatedResponse<WorkItem> = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.pagination_token,
            Some("next-token-abc".to_string())
        );
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].id, "wi-001");
        assert_eq!(response.data[0].status, "success");
        assert!(response.data[0].report_url.is_some());
        assert_eq!(response.data[1].id, "wi-002");
        assert_eq!(response.data[1].status, "pending");
        assert!(response.data[1].report_url.is_none());
    }

    #[test]
    fn test_paginated_workitem_response_no_token() {
        let json = r#"{
            "data": [
                {
                    "id": "wi-003",
                    "status": "inprogress",
                    "progress": "25%"
                }
            ]
        }"#;

        let response: PaginatedResponse<WorkItem> = serde_json::from_str(json).unwrap();
        assert!(response.pagination_token.is_none());
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].progress, Some("25%".to_string()));
    }

    #[test]
    fn test_workitem_full_stats_deserialization() {
        let json = r#"{
            "id": "wi-full",
            "status": "success",
            "reportUrl": "https://example.com/report.txt",
            "stats": {
                "timeQueued": "2024-01-01T00:00:00Z",
                "timeDownloadStarted": "2024-01-01T00:00:01Z",
                "timeInstructionStarted": "2024-01-01T00:00:02Z",
                "timeInstructionEnded": "2024-01-01T00:01:00Z",
                "timeUploadEnded": "2024-01-01T00:01:05Z",
                "timeFinished": "2024-01-01T00:01:06Z",
                "bytesDownloaded": 5242880,
                "bytesUploaded": 1048576
            }
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert_eq!(workitem.id, "wi-full");
        assert_eq!(workitem.status, "success");
        let stats = workitem.stats.unwrap();
        assert_eq!(stats.time_queued, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(stats.bytes_downloaded, Some(5242880));
        assert_eq!(stats.bytes_uploaded, Some(1048576));
        assert_eq!(
            stats.time_finished,
            Some("2024-01-01T00:01:06Z".to_string())
        );
    }
}

/// Integration tests using raps-mock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_mock_da_client(mock_url: &str) -> DesignAutomationClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: Some("test-nickname".to_string()),
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        DesignAutomationClient::new(config, auth)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_da_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_list_workitems() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_da_client(&server.url);
        let result = client.list_workitems().await;
        let _ = result;
    }
}
