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
use raps_kernel::http::HttpClientConfig;

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
    pub endpoint_url: String,
    pub form_data: std::collections::HashMap<String, String>,
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

    /// Get the nickname for this client (or "default")
    fn nickname(&self) -> &str {
        self.config.da_nickname.as_deref().unwrap_or("default")
    }

    /// List available engines
    ///
    /// Returns a list of engine IDs (e.g., "Autodesk.Revit+2024").
    /// Use `get_engine` to fetch full details for a specific engine.
    pub async fn list_engines(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/engines", self.config.da_url());

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list engines")?;

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

    /// List all app bundles
    pub async fn list_appbundles(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles", self.config.da_url());

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list appbundles")?;

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

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create appbundle")?;

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

    /// Delete an app bundle
    pub async fn delete_appbundle(&self, id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles/{}", self.config.da_url(), id);

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete appbundle")?;

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

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list activities")?;

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

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create activity")?;

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

    /// Delete an activity
    pub async fn delete_activity(&self, id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/activities/{}", self.config.da_url(), id);

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete activity")?;

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

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create workitem")?;

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

    /// Get work item status
    pub async fn get_workitem_status(&self, id: &str) -> Result<WorkItem> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/workitems/{}", self.config.da_url(), id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get workitem status")?;

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
}

/// Integration tests using wiremock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use wiremock::matchers::{header, method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Create a DA client configured to use the mock server
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

    /// Setup mock for 2-legged auth token
    async fn setup_auth_mock(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/authentication/v2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test-token-12345",
                "token_type": "Bearer",
                "expires_in": 3600
            })))
            .mount(server)
            .await;
    }

    // ==================== Engine Operations ====================

    #[tokio::test]
    async fn test_list_engines_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/da/us-east/v3/engines"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    "Autodesk.Revit+2024",
                    "Autodesk.AutoCAD+24"
                ],
                "paginationToken": null
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.list_engines().await;

        assert!(result.is_ok());
        let engines = result.unwrap();
        assert_eq!(engines.len(), 2);
        assert_eq!(engines[0], "Autodesk.Revit+2024");
    }

    #[tokio::test]
    async fn test_list_engines_empty() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/da/us-east/v3/engines"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [],
                "paginationToken": null
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.list_engines().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_engines_unauthorized() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/da/us-east/v3/engines"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "developerMessage": "Unauthorized"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.list_engines().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
    }

    // ==================== AppBundle Operations ====================

    #[tokio::test]
    async fn test_list_appbundles_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/da/us-east/v3/appbundles"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    "myapp.MyBundle+dev",
                    "myapp.AnotherBundle+prod"
                ],
                "paginationToken": null
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.list_appbundles().await;

        assert!(result.is_ok());
        let bundles = result.unwrap();
        assert_eq!(bundles.len(), 2);
        assert_eq!(bundles[0], "myapp.MyBundle+dev");
    }

    #[tokio::test]
    async fn test_create_appbundle_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/da/us-east/v3/appbundles"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-nickname.MyBundle+dev",
                "engine": "Autodesk.Revit+2024",
                "description": "Test bundle",
                "version": 1,
                "uploadParameters": {
                    "endpointUrl": "https://s3.amazonaws.com/upload",
                    "formData": {
                        "key": "value"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client
            .create_appbundle("MyBundle", "Autodesk.Revit+2024", Some("Test bundle"))
            .await;

        assert!(result.is_ok());
        let bundle = result.unwrap();
        assert_eq!(bundle.id, "test-nickname.MyBundle+dev");
        assert!(bundle.upload_parameters.is_some());
    }

    #[tokio::test]
    async fn test_create_appbundle_conflict() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/da/us-east/v3/appbundles"))
            .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
                "diagnostic": "AppBundle already exists"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client
            .create_appbundle("ExistingBundle", "Autodesk.Revit+2024", None)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("409"));
    }

    #[tokio::test]
    async fn test_delete_appbundle_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("DELETE"))
            .and(path_regex(r"/da/us-east/v3/appbundles/.+"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.delete_appbundle("test-nickname.MyBundle").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_appbundle_not_found() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("DELETE"))
            .and(path_regex(r"/da/us-east/v3/appbundles/.+"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "diagnostic": "AppBundle not found"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.delete_appbundle("nonexistent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    // ==================== Activity Operations ====================

    #[tokio::test]
    async fn test_list_activities_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/da/us-east/v3/activities"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    "myapp.ExtractData+dev",
                    "myapp.ExportDWG+prod"
                ],
                "paginationToken": null
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.list_activities().await;

        assert!(result.is_ok());
        let activities = result.unwrap();
        assert_eq!(activities.len(), 2);
    }

    #[tokio::test]
    async fn test_create_activity_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/da/us-east/v3/activities"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-nickname.MyActivity+dev",
                "engine": "Autodesk.Revit+2024",
                "description": "Test activity",
                "version": 1
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let request = CreateActivityRequest {
            id: "MyActivity".to_string(),
            engine: "Autodesk.Revit+2024".to_string(),
            command_line: vec!["$(engine.path)\\revitcoreconsole.exe".to_string()],
            app_bundles: vec!["test-nickname.MyBundle+dev".to_string()],
            parameters: std::collections::HashMap::new(),
            description: Some("Test activity".to_string()),
        };
        let result = client.create_activity(request).await;

        assert!(result.is_ok());
        let activity = result.unwrap();
        assert_eq!(activity.id, "test-nickname.MyActivity+dev");
    }

    #[tokio::test]
    async fn test_create_activity_invalid_engine() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/da/us-east/v3/activities"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "diagnostic": "Invalid engine specified"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let request = CreateActivityRequest {
            id: "MyActivity".to_string(),
            engine: "Invalid.Engine".to_string(),
            command_line: vec![],
            app_bundles: vec![],
            parameters: std::collections::HashMap::new(),
            description: None,
        };
        let result = client.create_activity(request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("400"));
    }

    #[tokio::test]
    async fn test_delete_activity_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("DELETE"))
            .and(path_regex(r"/da/us-east/v3/activities/.+"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.delete_activity("test-nickname.MyActivity").await;

        assert!(result.is_ok());
    }

    // ==================== WorkItem Operations ====================

    #[tokio::test]
    async fn test_create_workitem_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/da/us-east/v3/workitems"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "workitem-123",
                "status": "pending",
                "progress": "0%"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let mut arguments = std::collections::HashMap::new();
        arguments.insert(
            "input".to_string(),
            WorkItemArgument {
                url: "https://example.com/input.rvt".to_string(),
                verb: Some("get".to_string()),
                headers: None,
            },
        );
        let result = client
            .create_workitem("test-nickname.MyActivity+dev", arguments)
            .await;

        assert!(result.is_ok());
        let workitem = result.unwrap();
        assert_eq!(workitem.id, "workitem-123");
        assert_eq!(workitem.status, "pending");
    }

    #[tokio::test]
    async fn test_create_workitem_invalid_activity() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/da/us-east/v3/workitems"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "diagnostic": "Activity not found"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client
            .create_workitem("nonexistent.Activity", std::collections::HashMap::new())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    #[tokio::test]
    async fn test_get_workitem_status_pending() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/da/us-east/v3/workitems/.+"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "workitem-123",
                "status": "pending",
                "progress": "0%"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.get_workitem_status("workitem-123").await;

        assert!(result.is_ok());
        let workitem = result.unwrap();
        assert_eq!(workitem.status, "pending");
    }

    #[tokio::test]
    async fn test_get_workitem_status_inprogress() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/da/us-east/v3/workitems/.+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "workitem-123",
                "status": "inprogress",
                "progress": "50%"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.get_workitem_status("workitem-123").await;

        assert!(result.is_ok());
        let workitem = result.unwrap();
        assert_eq!(workitem.status, "inprogress");
        assert_eq!(workitem.progress, Some("50%".to_string()));
    }

    #[tokio::test]
    async fn test_get_workitem_status_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/da/us-east/v3/workitems/.+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "workitem-123",
                "status": "success",
                "progress": "100%",
                "reportUrl": "https://example.com/report.txt",
                "stats": {
                    "timeQueued": "2024-01-15T10:00:00Z",
                    "timeFinished": "2024-01-15T10:05:00Z",
                    "bytesDownloaded": 10240,
                    "bytesUploaded": 20480
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.get_workitem_status("workitem-123").await;

        assert!(result.is_ok());
        let workitem = result.unwrap();
        assert_eq!(workitem.status, "success");
        assert!(workitem.report_url.is_some());
        assert!(workitem.stats.is_some());
        let stats = workitem.stats.unwrap();
        assert_eq!(stats.bytes_downloaded, Some(10240));
    }

    #[tokio::test]
    async fn test_get_workitem_status_failed() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/da/us-east/v3/workitems/.+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "workitem-123",
                "status": "failed",
                "reportUrl": "https://example.com/error-report.txt"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.get_workitem_status("workitem-123").await;

        assert!(result.is_ok());
        let workitem = result.unwrap();
        assert_eq!(workitem.status, "failed");
    }

    #[tokio::test]
    async fn test_get_workitem_status_not_found() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/da/us-east/v3/workitems/.+"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "diagnostic": "WorkItem not found"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.get_workitem_status("nonexistent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    // ==================== Error Handling ====================

    #[tokio::test]
    async fn test_rate_limit_error() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/da/us-east/v3/engines"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "diagnostic": "Rate limit exceeded"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.list_engines().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("429"));
    }

    #[tokio::test]
    async fn test_server_error() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/da/us-east/v3/appbundles"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "diagnostic": "Internal server error"
            })))
            .mount(&server)
            .await;

        let client = create_mock_da_client(&server.uri());
        let result = client.list_appbundles().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("500"));
    }
}
