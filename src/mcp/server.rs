// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Server implementation for RAPS
//!
//! Exposes APS API functionality as MCP tools for AI assistants.

use rmcp::{model::*, transport::stdio, ServerHandler, ServiceExt};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::derivative::OutputFormat;
use crate::api::oss::Region;
use crate::api::{AuthClient, DataManagementClient, DerivativeClient, OssClient};
use crate::config::Config;
use crate::http::HttpClientConfig;

/// RAPS MCP Server
///
/// Provides AI assistants with direct access to Autodesk Platform Services.
#[derive(Clone)]
pub struct RapsServer {
    config: Arc<Config>,
    http_config: HttpClientConfig,
    // Cached clients
    auth_client: Arc<RwLock<Option<AuthClient>>>,
    oss_client: Arc<RwLock<Option<OssClient>>>,
    derivative_client: Arc<RwLock<Option<DerivativeClient>>>,
    dm_client: Arc<RwLock<Option<DataManagementClient>>>,
}

impl RapsServer {
    /// Create a new RAPS MCP Server
    pub fn new() -> Result<Self, anyhow::Error> {
        let config = Config::from_env()?;
        let http_config = HttpClientConfig::default();

        Ok(Self {
            config: Arc::new(config),
            http_config,
            auth_client: Arc::new(RwLock::new(None)),
            oss_client: Arc::new(RwLock::new(None)),
            derivative_client: Arc::new(RwLock::new(None)),
            dm_client: Arc::new(RwLock::new(None)),
        })
    }

    // Helper to get auth client
    async fn get_auth_client(&self) -> AuthClient {
        if let Some(client) = self.auth_client.read().await.clone() {
            return client;
        }

        let mut guard = self.auth_client.write().await;
        guard
            .get_or_insert_with(|| {
                AuthClient::new_with_http_config((*self.config).clone(), self.http_config.clone())
            })
            .clone()
    }

    // Helper to get OSS client
    async fn get_oss_client(&self) -> OssClient {
        if let Some(client) = self.oss_client.read().await.clone() {
            return client;
        }

        let auth = self.get_auth_client().await;
        let mut guard = self.oss_client.write().await;
        guard
            .get_or_insert_with(|| {
                OssClient::new_with_http_config(
                    (*self.config).clone(),
                    auth,
                    self.http_config.clone(),
                )
            })
            .clone()
    }

    // Helper to get Derivative client
    async fn get_derivative_client(&self) -> DerivativeClient {
        if let Some(client) = self.derivative_client.read().await.clone() {
            return client;
        }

        let auth = self.get_auth_client().await;
        let mut guard = self.derivative_client.write().await;
        guard
            .get_or_insert_with(|| {
                DerivativeClient::new_with_http_config(
                    (*self.config).clone(),
                    auth,
                    self.http_config.clone(),
                )
            })
            .clone()
    }

    // Helper to get Data Management client
    async fn get_dm_client(&self) -> DataManagementClient {
        if let Some(client) = self.dm_client.read().await.clone() {
            return client;
        }

        let auth = self.get_auth_client().await;
        let mut guard = self.dm_client.write().await;
        guard
            .get_or_insert_with(|| {
                DataManagementClient::new_with_http_config(
                    (*self.config).clone(),
                    auth,
                    self.http_config.clone(),
                )
            })
            .clone()
    }

    fn clamp_limit(limit: Option<usize>, default: usize, max: usize) -> usize {
        let limit = limit.unwrap_or(default).max(1);
        limit.min(max)
    }

    fn required_arg(args: &Map<String, Value>, key: &str) -> Result<String, String> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .ok_or_else(|| format!("❌ Missing required argument '{}'.", key))
    }

    fn optional_arg(args: &Map<String, Value>, key: &str) -> Option<String> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
    }

    // ========================================================================
    // Tool Implementations
    // ========================================================================

    async fn auth_test(&self) -> String {
        let auth = self.get_auth_client().await;
        match auth.get_token().await {
            Ok(_) => {
                "✅ Authentication successful! 2-legged OAuth credentials are valid.".to_string()
            }
            Err(e) => format!("❌ Authentication failed: {}", e),
        }
    }

    async fn auth_status(&self) -> String {
        let auth = self.get_auth_client().await;
        let mut status = String::new();

        // Check 2-legged
        match auth.get_token().await {
            Ok(_) => status.push_str("✅ 2-legged OAuth: Valid\n"),
            Err(_) => status.push_str("❌ 2-legged OAuth: Not configured or invalid\n"),
        }

        // Check 3-legged
        match auth.get_3leg_token().await {
            Ok(_) => status.push_str("✅ 3-legged OAuth: Valid (user logged in)\n"),
            Err(_) => status
                .push_str("⚠️ 3-legged OAuth: Not logged in (run 'raps auth login' to log in)\n"),
        }

        status
    }

    async fn bucket_list(&self, region: Option<String>, limit: Option<usize>) -> String {
        let client = self.get_oss_client().await;
        let limit = Self::clamp_limit(limit, 100, 500);

        match client.list_buckets().await {
            Ok(buckets) => {
                // Filter by region if specified
                let buckets: Vec<_> = buckets
                    .into_iter()
                    .filter(|b| {
                        if let Some(ref r) = region {
                            b.region
                                .as_ref()
                                .map(|br| br.eq_ignore_ascii_case(r))
                                .unwrap_or(true)
                        } else {
                            true
                        }
                    })
                    .take(limit)
                    .collect();

                // Format as simple output
                let mut output = format!("Found {} bucket(s):\n\n", buckets.len());
                for b in &buckets {
                    output.push_str(&format!(
                        "• {} (policy: {}, region: {})\n",
                        b.bucket_key,
                        b.policy_key,
                        b.region.as_deref().unwrap_or("unknown")
                    ));
                }
                output
            }
            Err(e) => format!("Error listing buckets: {}", e),
        }
    }

    async fn bucket_create(&self, bucket_key: String, policy: String, region: String) -> String {
        let client = self.get_oss_client().await;

        let retention = match policy.to_lowercase().as_str() {
            "transient" => crate::api::oss::RetentionPolicy::Transient,
            "temporary" => crate::api::oss::RetentionPolicy::Temporary,
            "persistent" => crate::api::oss::RetentionPolicy::Persistent,
            _ => {
                return "❌ Invalid policy. Use transient, temporary, or persistent.".to_string();
            }
        };

        let reg = match region.to_uppercase().as_str() {
            "EMEA" => Region::EMEA,
            "US" => Region::US,
            _ => return "❌ Invalid region. Use US or EMEA.".to_string(),
        };

        match client.create_bucket(&bucket_key, retention, reg).await {
            Ok(bucket) => format!(
                "✅ Bucket created successfully:\n• Key: {}\n• Owner: {}\n• Policy: {}",
                bucket.bucket_key, bucket.bucket_owner, bucket.policy_key
            ),
            Err(e) => format!("❌ Failed to create bucket: {}", e),
        }
    }

    async fn bucket_get(&self, bucket_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.get_bucket_details(&bucket_key).await {
            Ok(bucket) => format!(
                "Bucket: {}\n• Owner: {}\n• Policy: {}\n• Created: {}",
                bucket.bucket_key, bucket.bucket_owner, bucket.policy_key, bucket.created_date
            ),
            Err(e) => format!("❌ Bucket not found or error: {e}"),
        }
    }

    async fn bucket_delete(&self, bucket_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.delete_bucket(&bucket_key).await {
            Ok(()) => format!("✅ Bucket '{}' deleted successfully", bucket_key),
            Err(e) => format!("❌ Failed to delete bucket: {}", e),
        }
    }

    async fn object_list(&self, bucket_key: String, limit: Option<usize>) -> String {
        let client = self.get_oss_client().await;
        let limit = Self::clamp_limit(limit, 100, 1000);

        match client.list_objects(&bucket_key).await {
            Ok(objects) => {
                let objects: Vec<_> = objects.into_iter().take(limit).collect();
                let mut output =
                    format!("Found {} object(s) in '{}':\n\n", objects.len(), bucket_key);
                for obj in &objects {
                    output.push_str(&format!("• {} ({} bytes)\n", obj.object_key, obj.size));
                }
                output
            }
            Err(e) => format!("Error listing objects: {}", e),
        }
    }

    async fn object_delete(&self, bucket_key: String, object_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.delete_object(&bucket_key, &object_key).await {
            Ok(()) => format!(
                "✅ Object '{}' deleted from bucket '{}'",
                object_key, bucket_key
            ),
            Err(e) => format!("❌ Failed to delete object: {}", e),
        }
    }

    async fn object_signed_url(
        &self,
        bucket_key: String,
        object_key: String,
        minutes: u32,
    ) -> String {
        let client = self.get_oss_client().await;
        let minutes = minutes.clamp(2, 60);

        match client
            .get_signed_download_url(&bucket_key, &object_key, Some(minutes))
            .await
        {
            Ok(response) => {
                if let Some(url) = response.url {
                    format!(
                        "Pre-signed download URL (expires in {} minutes):\n{}",
                        minutes, url
                    )
                } else {
                    "No URL returned. The object may have been uploaded in chunks.".to_string()
                }
            }
            Err(e) => format!("❌ Failed to generate signed URL: {}", e),
        }
    }

    async fn object_urn(&self, bucket_key: String, object_key: String) -> String {
        let client = self.get_oss_client().await;
        let urn = client.get_urn(&bucket_key, &object_key);
        format!("URN for {}/{}:\n{}", bucket_key, object_key, urn)
    }

    async fn translate_start(&self, urn: String, format: String) -> String {
        let client = self.get_derivative_client().await;

        let output_format = match OutputFormat::from_str(&format) {
            Some(format) => format,
            None => {
                return "❌ Invalid output format. Supported: svf2, svf, thumbnail, obj, stl, step, iges, ifc.".to_string();
            }
        };

        match client.translate(&urn, output_format, None).await {
            Ok(result) => format!(
                "Translation job started:\n• Result: {}\n• URN: {}",
                result.result, result.urn
            ),
            Err(e) => format!("❌ Translation failed: {}", e),
        }
    }

    async fn translate_status(&self, urn: String) -> String {
        let client = self.get_derivative_client().await;

        match client.get_manifest(&urn).await {
            Ok(manifest) => {
                let status = &manifest.status;
                let progress = &manifest.progress;
                format!("Translation status: {} ({})", status, progress)
            }
            Err(e) => format!("❌ Could not get translation status: {}", e),
        }
    }

    async fn hub_list(&self, limit: Option<usize>) -> String {
        let client = self.get_dm_client().await;
        let limit = Self::clamp_limit(limit, 50, 200);

        match client.list_hubs().await {
            Ok(hubs) => {
                let hubs: Vec<_> = hubs.into_iter().take(limit).collect();
                let mut output = format!("Found {} hub(s):\n\n", hubs.len());
                for hub in &hubs {
                    let region = hub.attributes.region.as_deref().unwrap_or("unknown");
                    output.push_str(&format!(
                        "• {} (id: {}, region: {})\n",
                        hub.attributes.name, hub.id, region
                    ));
                }
                output
            }
            Err(e) => format!(
                "❌ Failed to list hubs (ensure you're logged in with 'raps auth login'): {}",
                e
            ),
        }
    }

    async fn project_list(&self, hub_id: String, limit: Option<usize>) -> String {
        let client = self.get_dm_client().await;
        let limit = Self::clamp_limit(limit, 50, 200);

        match client.list_projects(&hub_id).await {
            Ok(projects) => {
                let projects: Vec<_> = projects.into_iter().take(limit).collect();
                let mut output = format!("Found {} project(s):\n\n", projects.len());
                for proj in &projects {
                    output.push_str(&format!("• {} (id: {})\n", proj.attributes.name, proj.id));
                }
                output
            }
            Err(e) => format!("❌ Failed to list projects: {}", e),
        }
    }

    // Tool dispatch
    async fn dispatch_tool(&self, name: &str, args: Option<Map<String, Value>>) -> CallToolResult {
        let args = args.unwrap_or_default();

        let result = match name {
            "auth_test" => self.auth_test().await,
            "auth_status" => self.auth_status().await,
            "bucket_list" => {
                let region = Self::optional_arg(&args, "region");
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.bucket_list(region, limit).await
            }
            "bucket_create" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let policy =
                    Self::optional_arg(&args, "policy").unwrap_or_else(|| "transient".to_string());
                let region =
                    Self::optional_arg(&args, "region").unwrap_or_else(|| "US".to_string());
                self.bucket_create(bucket_key, policy, region).await
            }
            "bucket_get" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.bucket_get(bucket_key).await
            }
            "bucket_delete" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.bucket_delete(bucket_key).await
            }
            "object_list" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.object_list(bucket_key, limit).await
            }
            "object_delete" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let object_key = match Self::required_arg(&args, "object_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.object_delete(bucket_key, object_key).await
            }
            "object_signed_url" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let object_key = match Self::required_arg(&args, "object_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let minutes = args.get("minutes").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
                self.object_signed_url(bucket_key, object_key, minutes)
                    .await
            }
            "object_urn" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let object_key = match Self::required_arg(&args, "object_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.object_urn(bucket_key, object_key).await
            }
            "translate_start" => {
                let urn = match Self::required_arg(&args, "urn") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let format =
                    Self::optional_arg(&args, "format").unwrap_or_else(|| "svf2".to_string());
                self.translate_start(urn, format).await
            }
            "translate_status" => {
                let urn = match Self::required_arg(&args, "urn") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.translate_status(urn).await
            }
            "hub_list" => {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.hub_list(limit).await
            }
            "project_list" => {
                let hub_id = match Self::required_arg(&args, "hub_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.project_list(hub_id, limit).await
            }
            _ => format!("Unknown tool: {}", name),
        };

        CallToolResult::success(vec![Content::text(result)])
    }
}

// Helper to create tool schema
fn schema(props: Value, required: &[&str]) -> Arc<Map<String, Value>> {
    let mut obj = Map::new();
    obj.insert("type".to_string(), json!("object"));
    obj.insert("properties".to_string(), props);
    obj.insert("required".to_string(), json!(required));
    Arc::new(obj)
}

// Tool definitions
fn get_tools() -> Vec<Tool> {
    vec![
        Tool::new(
            "auth_test",
            "Test 2-legged OAuth authentication with APS",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "auth_status",
            "Check authentication status (2-legged and 3-legged)",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "bucket_list",
            "List OSS buckets. Buckets are containers for storing files.",
            schema(
                json!({
                    "region": {"type": "string", "description": "Filter by region: US or EMEA"},
                    "limit": {"type": "integer", "description": "Max buckets (default: 100)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "bucket_create",
            "Create a new OSS bucket. Keys must be globally unique, 3-128 chars.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Unique bucket key"},
                    "policy": {"type": "string", "description": "transient (24h), temporary (30d), or persistent"},
                    "region": {"type": "string", "description": "US or EMEA (default: US)"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "bucket_get",
            "Get detailed bucket information",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "bucket_delete",
            "Delete an OSS bucket (must be empty)",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Bucket key to delete"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "object_list",
            "List objects (files) in an OSS bucket",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "limit": {"type": "integer", "description": "Max objects (default: 100)"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "object_delete",
            "Delete an object from an OSS bucket",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "object_key": {"type": "string", "description": "Object key (filename)"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "object_signed_url",
            "Generate pre-signed S3 URL for direct download",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "object_key": {"type": "string", "description": "The object key"},
                    "minutes": {"type": "integer", "description": "Expiry (2-60 min, default: 10)"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "object_urn",
            "Get Base64-encoded URN for an object (used for translation)",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "object_key": {"type": "string", "description": "The object key"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "translate_start",
            "Start CAD translation. Formats: svf2, obj, stl, step, iges, ifc",
            schema(
                json!({
                    "urn": {"type": "string", "description": "Base64-encoded URN"},
                    "format": {"type": "string", "description": "Output format (default: svf2)"}
                }),
                &["urn"],
            ),
        ),
        Tool::new(
            "translate_status",
            "Check translation status: pending, inprogress, success, failed",
            schema(
                json!({
                    "urn": {"type": "string", "description": "Base64-encoded URN"}
                }),
                &["urn"],
            ),
        ),
        Tool::new(
            "hub_list",
            "List accessible hubs (BIM 360/ACC). Requires 3-legged auth.",
            schema(
                json!({
                    "limit": {"type": "integer", "description": "Max hubs (default: 50)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "project_list",
            "List projects in a hub. Requires 3-legged auth.",
            schema(
                json!({
                    "hub_id": {"type": "string", "description": "The hub ID"},
                    "limit": {"type": "integer", "description": "Max projects (default: 50)"}
                }),
                &["hub_id"],
            ),
        ),
    ]
}

// ServerHandler implementation
impl ServerHandler for RapsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "🌼 RAPS MCP Server - Autodesk Platform Services CLI\n\n\
                Provides direct access to APS APIs:\n\
                • auth_test, auth_status - Authentication\n\
                • bucket_* - OSS storage buckets\n\
                • object_* - Files in buckets\n\
                • translate_* - CAD translation\n\
                • hub_list, project_list - BIM 360/ACC data\n\n\
                Set APS_CLIENT_ID and APS_CLIENT_SECRET env vars."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult {
            tools: get_tools(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let result = self.dispatch_tool(&request.name, request.arguments).await;
        Ok(result)
    }
}

/// Run the MCP server using stdio transport
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debugging (optional, outputs to stderr)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let server = RapsServer::new()?;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
