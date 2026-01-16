// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Server implementation for RAPS
//!
//! Exposes APS API functionality as MCP tools for AI assistants.

use rmcp::{ServerHandler, ServiceExt, model::*, transport::stdio};
use serde_json::{Map, Value, json};
use std::{str::FromStr, sync::Arc};
use tokio::sync::RwLock;

use raps_acc::{
    AccClient, CreateIssueRequest, IssuesClient, RfiClient, UpdateIssueRequest,
    admin::AccountAdminClient, users::ProjectUsersClient,
};
use raps_admin::{BulkConfig, ProjectFilter, StateManager};
use raps_derivative::{DerivativeClient, OutputFormat};
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;
use raps_oss::{OssClient, Region, RetentionPolicy};

/// RAPS MCP Server
///
/// Provides AI assistants with direct access to Autodesk Platform Services.
#[derive(Clone)]
pub struct RapsServer {
    config: Arc<Config>,
    http_config: HttpClientConfig,
    // Cached clients (Clone-able)
    auth_client: Arc<RwLock<Option<AuthClient>>>,
    oss_client: Arc<RwLock<Option<OssClient>>>,
    derivative_client: Arc<RwLock<Option<DerivativeClient>>>,
    dm_client: Arc<RwLock<Option<DataManagementClient>>>,
    // Note: ACC/Admin clients are created on-demand (not cached) as they don't implement Clone
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

    // Helper to get Account Admin client (created on demand, not cached)
    async fn get_admin_client(&self) -> AccountAdminClient {
        let auth = self.get_auth_client().await;
        AccountAdminClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
    }

    // Helper to get Project Users client (created on demand, not cached)
    async fn get_users_client(&self) -> ProjectUsersClient {
        let auth = self.get_auth_client().await;
        ProjectUsersClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
    }

    // Helper to get Issues client (created on demand, not cached)
    async fn get_issues_client(&self) -> IssuesClient {
        let auth = self.get_auth_client().await;
        IssuesClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
    }

    // Helper to get RFI client (created on demand, not cached)
    async fn get_rfi_client(&self) -> RfiClient {
        let auth = self.get_auth_client().await;
        RfiClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
    }

    // Helper to get ACC Extended client (created on demand, not cached)
    async fn get_acc_client(&self) -> AccClient {
        let auth = self.get_auth_client().await;
        AccClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
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
            .ok_or_else(|| format!("Missing required argument '{}'.", key))
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
            Ok(_) => "Authentication successful! 2-legged OAuth credentials are valid.".to_string(),
            Err(e) => format!("Authentication failed: {}", e),
        }
    }

    async fn auth_status(&self) -> String {
        let auth = self.get_auth_client().await;
        let mut status = String::new();

        // Check 2-legged
        match auth.get_token().await {
            Ok(_) => status.push_str("2-legged OAuth: Valid\n"),
            Err(_) => status.push_str("2-legged OAuth: Not configured or invalid\n"),
        }

        // Check 3-legged
        match auth.get_3leg_token().await {
            Ok(_) => status.push_str("3-legged OAuth: Valid (user logged in)\n"),
            Err(_) => {
                status.push_str("3-legged OAuth: Not logged in (run 'raps auth login' to log in)\n")
            }
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
                        "* {} (policy: {}, region: {})\n",
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
            "transient" => RetentionPolicy::Transient,
            "temporary" => RetentionPolicy::Temporary,
            "persistent" => RetentionPolicy::Persistent,
            _ => {
                return "Invalid policy. Use transient, temporary, or persistent.".to_string();
            }
        };

        let reg = match region.to_uppercase().as_str() {
            "EMEA" => Region::EMEA,
            "US" => Region::US,
            _ => return "Invalid region. Use US or EMEA.".to_string(),
        };

        match client.create_bucket(&bucket_key, retention, reg).await {
            Ok(bucket) => format!(
                "Bucket created successfully:\n* Key: {}\n* Owner: {}\n* Policy: {}",
                bucket.bucket_key, bucket.bucket_owner, bucket.policy_key
            ),
            Err(e) => format!("Failed to create bucket: {}", e),
        }
    }

    async fn bucket_get(&self, bucket_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.get_bucket_details(&bucket_key).await {
            Ok(bucket) => format!(
                "Bucket: {}\n* Owner: {}\n* Policy: {}\n* Created: {}",
                bucket.bucket_key, bucket.bucket_owner, bucket.policy_key, bucket.created_date
            ),
            Err(e) => format!("Bucket not found or error: {e}"),
        }
    }

    async fn bucket_delete(&self, bucket_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.delete_bucket(&bucket_key).await {
            Ok(()) => format!("Bucket '{}' deleted successfully", bucket_key),
            Err(e) => format!("Failed to delete bucket: {}", e),
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
                    output.push_str(&format!("* {} ({} bytes)\n", obj.object_key, obj.size));
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
                "Object '{}' deleted from bucket '{}'",
                object_key, bucket_key
            ),
            Err(e) => format!("Failed to delete object: {}", e),
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
            Err(e) => format!("Failed to generate signed URL: {}", e),
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
            Ok(format) => format,
            Err(_) => {
                return "Invalid output format. Supported: svf2, svf, thumbnail, obj, stl, step, iges, ifc.".to_string();
            }
        };

        match client.translate(&urn, output_format, None).await {
            Ok(result) => format!(
                "Translation job started:\n* Result: {}\n* URN: {}",
                result.result, result.urn
            ),
            Err(e) => format!("Translation failed: {}", e),
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
            Err(e) => format!("Could not get translation status: {}", e),
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
                        "* {} (id: {}, region: {})\n",
                        hub.attributes.name, hub.id, region
                    ));
                }
                output
            }
            Err(e) => format!(
                "Failed to list hubs (ensure you're logged in with 'raps auth login'): {}",
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
                    output.push_str(&format!("* {} (id: {})\n", proj.attributes.name, proj.id));
                }
                output
            }
            Err(e) => format!("Failed to list projects: {}", e),
        }
    }

    // ========================================================================
    // Admin Tools - Account Admin Bulk Operations (v4.0)
    // ========================================================================

    async fn admin_project_list(
        &self,
        account_id: String,
        filter: Option<String>,
        limit: Option<usize>,
    ) -> String {
        let client = self.get_admin_client().await;
        let limit = Self::clamp_limit(limit, 100, 500);

        match client.list_all_projects(&account_id).await {
            Ok(projects) => {
                // Apply filter if specified
                let project_filter = if let Some(ref f) = filter {
                    match ProjectFilter::from_expression(f) {
                        Ok(pf) => pf,
                        Err(e) => return format!("Invalid filter expression: {}", e),
                    }
                } else {
                    ProjectFilter::new()
                };

                let filtered = project_filter.apply(projects);
                let filtered: Vec<_> = filtered.into_iter().take(limit).collect();

                let mut output = format!("Found {} project(s) in account {}:\n\n", filtered.len(), account_id);
                for proj in &filtered {
                    let status = proj.status.as_deref().unwrap_or("unknown");
                    let platform = if proj.is_acc() { "ACC" } else if proj.is_bim360() { "BIM360" } else { "unknown" };
                    output.push_str(&format!("* {} (id: {}, status: {}, platform: {})\n",
                        proj.name, proj.id, status, platform));
                }
                output
            }
            Err(e) => format!("Failed to list projects: {}", e),
        }
    }

    async fn admin_user_add(
        &self,
        account_id: String,
        email: String,
        role: Option<String>,
        filter: Option<String>,
        dry_run: bool,
    ) -> String {
        let admin_client = self.get_admin_client().await;
        let users_client = Arc::new(self.get_users_client().await);

        // Parse filter
        let project_filter = if let Some(ref f) = filter {
            match ProjectFilter::from_expression(f) {
                Ok(pf) => pf,
                Err(e) => return format!("Invalid filter expression: {}", e),
            }
        } else {
            ProjectFilter::new()
        };

        let bulk_config = BulkConfig {
            concurrency: 10,
            dry_run,
            ..Default::default()
        };

        // Progress callback (no-op for MCP)
        let on_progress = |_| {};

        match raps_admin::bulk_add_user(
            &admin_client,
            users_client,
            &account_id,
            &email,
            role.as_deref(),
            &project_filter,
            bulk_config,
            on_progress,
        ).await {
            Ok(result) => {
                let mut output = format!(
                    "Bulk add user operation {}:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s\n",
                    if dry_run { "(DRY RUN)" } else { "completed" },
                    result.total,
                    result.completed,
                    result.skipped,
                    result.failed,
                    result.duration.as_secs_f64()
                );
                if result.failed > 0 {
                    output.push_str("\nFailed projects:\n");
                    for detail in &result.details {
                        if let raps_admin::ItemResult::Failed { error, .. } = &detail.result {
                            output.push_str(&format!("  * {}: {}\n",
                                detail.project_name.as_deref().unwrap_or(&detail.project_id),
                                error));
                        }
                    }
                }
                output
            }
            Err(e) => format!("Bulk add user failed: {}", e),
        }
    }

    async fn admin_user_remove(
        &self,
        account_id: String,
        email: String,
        filter: Option<String>,
        dry_run: bool,
    ) -> String {
        let admin_client = self.get_admin_client().await;
        let users_client = Arc::new(self.get_users_client().await);

        let project_filter = if let Some(ref f) = filter {
            match ProjectFilter::from_expression(f) {
                Ok(pf) => pf,
                Err(e) => return format!("Invalid filter expression: {}", e),
            }
        } else {
            ProjectFilter::new()
        };

        let bulk_config = BulkConfig {
            concurrency: 10,
            dry_run,
            ..Default::default()
        };

        let on_progress = |_| {};

        match raps_admin::bulk_remove_user(
            &admin_client,
            users_client,
            &account_id,
            &email,
            &project_filter,
            bulk_config,
            on_progress,
        ).await {
            Ok(result) => {
                format!(
                    "Bulk remove user operation {}:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s",
                    if dry_run { "(DRY RUN)" } else { "completed" },
                    result.total,
                    result.completed,
                    result.skipped,
                    result.failed,
                    result.duration.as_secs_f64()
                )
            }
            Err(e) => format!("Bulk remove user failed: {}", e),
        }
    }

    async fn admin_user_update_role(
        &self,
        account_id: String,
        email: String,
        role: String,
        filter: Option<String>,
        dry_run: bool,
    ) -> String {
        let admin_client = self.get_admin_client().await;
        let users_client = Arc::new(self.get_users_client().await);

        let project_filter = if let Some(ref f) = filter {
            match ProjectFilter::from_expression(f) {
                Ok(pf) => pf,
                Err(e) => return format!("Invalid filter expression: {}", e),
            }
        } else {
            ProjectFilter::new()
        };

        let bulk_config = BulkConfig {
            concurrency: 10,
            dry_run,
            ..Default::default()
        };

        let on_progress = |_| {};

        match raps_admin::bulk_update_role(
            &admin_client,
            users_client,
            &account_id,
            &email,
            &role,
            None, // from_role
            &project_filter,
            bulk_config,
            on_progress,
        ).await {
            Ok(result) => {
                format!(
                    "Bulk update role operation {}:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s",
                    if dry_run { "(DRY RUN)" } else { "completed" },
                    result.total,
                    result.completed,
                    result.skipped,
                    result.failed,
                    result.duration.as_secs_f64()
                )
            }
            Err(e) => format!("Bulk update role failed: {}", e),
        }
    }

    async fn admin_operation_list(&self, limit: Option<usize>) -> String {
        let limit = Self::clamp_limit(limit, 10, 50);

        match StateManager::new() {
            Ok(state_manager) => {
                match state_manager.list_operations(None).await {
                    Ok(operations) => {
                        let operations: Vec<_> = operations.into_iter().take(limit).collect();
                        if operations.is_empty() {
                            return "No operations found.".to_string();
                        }

                        let mut output = format!("Found {} operation(s):\n\n", operations.len());
                        for op in &operations {
                            output.push_str(&format!(
                                "* {} ({:?}) - {:?} [{}/{}]\n",
                                op.operation_id,
                                op.operation_type,
                                op.status,
                                op.completed + op.skipped + op.failed,
                                op.total
                            ));
                        }
                        output
                    }
                    Err(e) => format!("Failed to list operations: {}", e),
                }
            }
            Err(e) => format!("Failed to initialize state manager: {}", e),
        }
    }

    async fn admin_operation_status(&self, operation_id: Option<String>) -> String {
        match StateManager::new() {
            Ok(state_manager) => {
                let op_id = match operation_id {
                    Some(id) => match uuid::Uuid::parse_str(&id) {
                        Ok(uuid) => uuid,
                        Err(_) => return "Invalid operation ID format".to_string(),
                    },
                    None => {
                        // Get most recent
                        match state_manager.list_operations(None).await {
                            Ok(ops) if !ops.is_empty() => ops[0].operation_id,
                            _ => return "No operations found".to_string(),
                        }
                    }
                };

                match state_manager.load_operation(op_id).await {
                    Ok(state) => {
                        let completed = state.results.values()
                            .filter(|r| matches!(r.result, raps_admin::ItemResult::Success))
                            .count();
                        let skipped = state.results.values()
                            .filter(|r| matches!(r.result, raps_admin::ItemResult::Skipped { .. }))
                            .count();
                        let failed = state.results.values()
                            .filter(|r| matches!(r.result, raps_admin::ItemResult::Failed { .. }))
                            .count();

                        format!(
                            "Operation Status:\n\n* ID: {}\n* Type: {:?}\n* Status: {:?}\n* Progress: {}/{}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Created: {}\n* Updated: {}",
                            state.operation_id,
                            state.operation_type,
                            state.status,
                            completed + skipped + failed,
                            state.project_ids.len(),
                            completed,
                            skipped,
                            failed,
                            state.created_at.to_rfc3339(),
                            state.updated_at.to_rfc3339()
                        )
                    }
                    Err(e) => format!("Failed to load operation: {}", e),
                }
            }
            Err(e) => format!("Failed to initialize state manager: {}", e),
        }
    }

    // ========================================================================
    // Folder/Item Management Tools
    // ========================================================================

    async fn folder_list(&self, project_id: String, folder_id: String) -> String {
        let client = self.get_dm_client().await;

        match client.list_folder_contents(&project_id, &folder_id).await {
            Ok(contents) => {
                if contents.is_empty() {
                    return "Folder is empty.".to_string();
                }

                let mut output = format!("Found {} item(s):\n\n", contents.len());
                for item in &contents {
                    let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
                    let name = item.get("attributes")
                        .and_then(|a| a.get("displayName").or(a.get("name")))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unnamed");
                    let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("unknown");
                    let icon = if item_type == "folders" { "[folder]" } else { "[file]" };
                    output.push_str(&format!("* {} {} (id: {})\n", icon, name, id));
                }
                output
            }
            Err(e) => format!("Failed to list folder contents: {}", e),
        }
    }

    async fn folder_create(&self, project_id: String, parent_folder_id: String, name: String) -> String {
        let client = self.get_dm_client().await;

        match client.create_folder(&project_id, &parent_folder_id, &name).await {
            Ok(folder) => format!(
                "Folder created successfully:\n* Name: {}\n* ID: {}",
                folder.attributes.name, folder.id
            ),
            Err(e) => format!("Failed to create folder: {}", e),
        }
    }

    async fn item_info(&self, project_id: String, item_id: String) -> String {
        let client = self.get_dm_client().await;

        match client.get_item(&project_id, &item_id).await {
            Ok(item) => {
                let mut output = format!(
                    "Item Details:\n\n* Name: {}\n* ID: {}\n* Type: {}",
                    item.attributes.display_name,
                    item.id,
                    item.item_type
                );
                if let Some(ref create_time) = item.attributes.create_time {
                    output.push_str(&format!("\n* Created: {}", create_time));
                }
                if let Some(ref modified_time) = item.attributes.last_modified_time {
                    output.push_str(&format!("\n* Modified: {}", modified_time));
                }
                output
            }
            Err(e) => format!("Failed to get item: {}", e),
        }
    }

    async fn item_versions(&self, project_id: String, item_id: String) -> String {
        let client = self.get_dm_client().await;

        match client.get_item_versions(&project_id, &item_id).await {
            Ok(versions) => {
                if versions.is_empty() {
                    return "No versions found.".to_string();
                }

                let mut output = format!("Found {} version(s):\n\n", versions.len());
                for v in &versions {
                    let ver_num = v.attributes.version_number
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let name = v.attributes.display_name.as_ref()
                        .or(Some(&v.attributes.name))
                        .map(|s| s.as_str())
                        .unwrap_or("-");
                    output.push_str(&format!("* v{}: {}\n", ver_num, name));
                }
                output
            }
            Err(e) => format!("Failed to get item versions: {}", e),
        }
    }

    // ========================================================================
    // Issues Tools
    // ========================================================================

    async fn issue_list(&self, project_id: String, filter: Option<String>) -> String {
        let client = self.get_issues_client().await;

        match client.list_issues(&project_id, filter.as_deref()).await {
            Ok(issues) => {
                if issues.is_empty() {
                    return "No issues found.".to_string();
                }

                let mut output = format!("Found {} issue(s):\n\n", issues.len());
                for issue in &issues {
                    let display_id = issue.display_id.map(|d| d.to_string()).unwrap_or_else(|| "-".to_string());
                    output.push_str(&format!(
                        "* #{} {} [{}]\n  ID: {}\n",
                        display_id, issue.title, issue.status, issue.id
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list issues: {}", e),
        }
    }

    async fn issue_get(&self, project_id: String, issue_id: String) -> String {
        let client = self.get_issues_client().await;

        match client.get_issue(&project_id, &issue_id).await {
            Ok(issue) => {
                let mut output = format!(
                    "Issue Details:\n\n* Title: {}\n* Status: {}\n* ID: {}",
                    issue.title, issue.status, issue.id
                );
                if let Some(ref desc) = issue.description {
                    output.push_str(&format!("\n* Description: {}", desc));
                }
                if let Some(ref assigned) = issue.assigned_to {
                    output.push_str(&format!("\n* Assigned to: {}", assigned));
                }
                if let Some(ref due) = issue.due_date {
                    output.push_str(&format!("\n* Due date: {}", due));
                }
                output
            }
            Err(e) => format!("Failed to get issue: {}", e),
        }
    }

    async fn issue_create(
        &self,
        project_id: String,
        title: String,
        description: Option<String>,
        status: Option<String>,
    ) -> String {
        let client = self.get_issues_client().await;

        let request = CreateIssueRequest {
            title,
            description,
            status: status.unwrap_or_else(|| "open".to_string()),
            issue_type_id: None,
            issue_subtype_id: None,
            assigned_to: None,
            assigned_to_type: None,
            due_date: None,
        };

        match client.create_issue(&project_id, request).await {
            Ok(issue) => format!(
                "Issue created successfully:\n* Title: {}\n* ID: {}\n* Status: {}",
                issue.title, issue.id, issue.status
            ),
            Err(e) => format!("Failed to create issue: {}", e),
        }
    }

    async fn issue_update(
        &self,
        project_id: String,
        issue_id: String,
        title: Option<String>,
        description: Option<String>,
        status: Option<String>,
    ) -> String {
        let client = self.get_issues_client().await;

        let request = UpdateIssueRequest {
            title,
            description,
            status,
            assigned_to: None,
            due_date: None,
        };

        match client.update_issue(&project_id, &issue_id, request).await {
            Ok(issue) => format!(
                "Issue updated successfully:\n* Title: {}\n* Status: {}",
                issue.title, issue.status
            ),
            Err(e) => format!("Failed to update issue: {}", e),
        }
    }

    // ========================================================================
    // RFI Tools
    // ========================================================================

    async fn rfi_list(&self, project_id: String) -> String {
        let client = self.get_rfi_client().await;

        match client.list_rfis(&project_id).await {
            Ok(rfis) => {
                if rfis.is_empty() {
                    return "No RFIs found.".to_string();
                }

                let mut output = format!("Found {} RFI(s):\n\n", rfis.len());
                for rfi in &rfis {
                    let number = rfi.number.as_deref().unwrap_or("-");
                    output.push_str(&format!(
                        "* #{} {} [{}]\n  ID: {}\n",
                        number, rfi.title, rfi.status, rfi.id
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list RFIs: {}", e),
        }
    }

    async fn rfi_get(&self, project_id: String, rfi_id: String) -> String {
        let client = self.get_rfi_client().await;

        match client.get_rfi(&project_id, &rfi_id).await {
            Ok(rfi) => {
                let mut output = format!(
                    "RFI Details:\n\n* Title: {}\n* Status: {}\n* ID: {}",
                    rfi.title, rfi.status, rfi.id
                );
                if let Some(ref question) = rfi.question {
                    output.push_str(&format!("\n* Question: {}", question));
                }
                if let Some(ref answer) = rfi.answer {
                    output.push_str(&format!("\n* Answer: {}", answer));
                }
                if let Some(ref priority) = rfi.priority {
                    output.push_str(&format!("\n* Priority: {}", priority));
                }
                output
            }
            Err(e) => format!("Failed to get RFI: {}", e),
        }
    }

    // ========================================================================
    // ACC Extended Tools (Assets, Submittals, Checklists)
    // ========================================================================

    async fn acc_assets_list(&self, project_id: String) -> String {
        let client = self.get_acc_client().await;

        match client.list_assets(&project_id).await {
            Ok(assets) => {
                if assets.is_empty() {
                    return "No assets found.".to_string();
                }

                let mut output = format!("Found {} asset(s):\n\n", assets.len());
                for asset in &assets {
                    let desc = asset.description.as_deref().unwrap_or("-");
                    output.push_str(&format!("* {} - {}\n", asset.id, desc));
                }
                output
            }
            Err(e) => format!("Failed to list assets: {}", e),
        }
    }

    async fn acc_submittals_list(&self, project_id: String) -> String {
        let client = self.get_acc_client().await;

        match client.list_submittals(&project_id).await {
            Ok(submittals) => {
                if submittals.is_empty() {
                    return "No submittals found.".to_string();
                }

                let mut output = format!("Found {} submittal(s):\n\n", submittals.len());
                for sub in &submittals {
                    let number = sub.number.as_deref().unwrap_or("-");
                    output.push_str(&format!(
                        "* #{} {} [{}]\n  ID: {}\n",
                        number, sub.title, sub.status, sub.id
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list submittals: {}", e),
        }
    }

    async fn acc_checklists_list(&self, project_id: String) -> String {
        let client = self.get_acc_client().await;

        match client.list_checklists(&project_id).await {
            Ok(checklists) => {
                if checklists.is_empty() {
                    return "No checklists found.".to_string();
                }

                let mut output = format!("Found {} checklist(s):\n\n", checklists.len());
                for checklist in &checklists {
                    output.push_str(&format!(
                        "* {} [{}]\n  ID: {}\n",
                        checklist.title, checklist.status, checklist.id
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list checklists: {}", e),
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

            // ================================================================
            // Admin Tools
            // ================================================================
            "admin_project_list" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let filter = Self::optional_arg(&args, "filter");
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);
                self.admin_project_list(account_id, filter, limit).await
            }
            "admin_user_add" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let email = match Self::required_arg(&args, "email") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let role = Self::optional_arg(&args, "role");
                let filter = Self::optional_arg(&args, "filter");
                let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
                self.admin_user_add(account_id, email, role, filter, dry_run).await
            }
            "admin_user_remove" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let email = match Self::required_arg(&args, "email") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let filter = Self::optional_arg(&args, "filter");
                let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
                self.admin_user_remove(account_id, email, filter, dry_run).await
            }
            "admin_user_update_role" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let email = match Self::required_arg(&args, "email") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let role = match Self::required_arg(&args, "role") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let filter = Self::optional_arg(&args, "filter");
                let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
                self.admin_user_update_role(account_id, email, role, filter, dry_run).await
            }
            "admin_operation_list" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);
                self.admin_operation_list(limit).await
            }
            "admin_operation_status" => {
                let operation_id = Self::optional_arg(&args, "operation_id");
                self.admin_operation_status(operation_id).await
            }

            // ================================================================
            // Folder/Item Tools
            // ================================================================
            "folder_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let folder_id = match Self::required_arg(&args, "folder_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.folder_list(project_id, folder_id).await
            }
            "folder_create" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let parent_folder_id = match Self::required_arg(&args, "parent_folder_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let name = match Self::required_arg(&args, "name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.folder_create(project_id, parent_folder_id, name).await
            }
            "item_info" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let item_id = match Self::required_arg(&args, "item_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.item_info(project_id, item_id).await
            }
            "item_versions" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let item_id = match Self::required_arg(&args, "item_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.item_versions(project_id, item_id).await
            }

            // ================================================================
            // Issues Tools
            // ================================================================
            "issue_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let filter = Self::optional_arg(&args, "filter");
                self.issue_list(project_id, filter).await
            }
            "issue_get" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let issue_id = match Self::required_arg(&args, "issue_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.issue_get(project_id, issue_id).await
            }
            "issue_create" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = match Self::required_arg(&args, "title") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let description = Self::optional_arg(&args, "description");
                let status = Self::optional_arg(&args, "status");
                self.issue_create(project_id, title, description, status).await
            }
            "issue_update" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let issue_id = match Self::required_arg(&args, "issue_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = Self::optional_arg(&args, "title");
                let description = Self::optional_arg(&args, "description");
                let status = Self::optional_arg(&args, "status");
                self.issue_update(project_id, issue_id, title, description, status).await
            }

            // ================================================================
            // RFI Tools
            // ================================================================
            "rfi_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.rfi_list(project_id).await
            }
            "rfi_get" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let rfi_id = match Self::required_arg(&args, "rfi_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.rfi_get(project_id, rfi_id).await
            }

            // ================================================================
            // ACC Extended Tools
            // ================================================================
            "acc_assets_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.acc_assets_list(project_id).await
            }
            "acc_submittals_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.acc_submittals_list(project_id).await
            }
            "acc_checklists_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.acc_checklists_list(project_id).await
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

        // ================================================================
        // Admin Tools (v4.0 - Bulk Operations)
        // ================================================================
        Tool::new(
            "admin_project_list",
            "List all projects in an ACC/BIM360 account with filtering. Use 2-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "filter": {"type": "string", "description": "Filter expression (e.g., 'name:~Active', 'status:active', 'platform:acc')"},
                    "limit": {"type": "integer", "description": "Max projects (default: 100)"}
                }),
                &["account_id"],
            ),
        ),
        Tool::new(
            "admin_user_add",
            "Bulk add a user to multiple projects across an account. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to add"},
                    "role": {"type": "string", "description": "Role: project_admin, viewer, editor, etc."},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview without making changes (default: false)"}
                }),
                &["account_id", "email"],
            ),
        ),
        Tool::new(
            "admin_user_remove",
            "Bulk remove a user from multiple projects across an account. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to remove"},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview without making changes (default: false)"}
                }),
                &["account_id", "email"],
            ),
        ),
        Tool::new(
            "admin_user_update_role",
            "Bulk update a user's role across multiple projects. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to update"},
                    "role": {"type": "string", "description": "New role: project_admin, viewer, editor, etc."},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview without making changes (default: false)"}
                }),
                &["account_id", "email", "role"],
            ),
        ),
        Tool::new(
            "admin_operation_list",
            "List recent bulk admin operations for status tracking and resume.",
            schema(
                json!({
                    "limit": {"type": "integer", "description": "Max operations (default: 10)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "admin_operation_status",
            "Get detailed status of a bulk admin operation.",
            schema(
                json!({
                    "operation_id": {"type": "string", "description": "Operation ID (optional, defaults to most recent)"}
                }),
                &[],
            ),
        ),

        // ================================================================
        // Folder/Item Tools
        // ================================================================
        Tool::new(
            "folder_list",
            "List contents of a folder (files and subfolders). Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "folder_id": {"type": "string", "description": "The folder ID"}
                }),
                &["project_id", "folder_id"],
            ),
        ),
        Tool::new(
            "folder_create",
            "Create a new folder in a project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "parent_folder_id": {"type": "string", "description": "Parent folder ID"},
                    "name": {"type": "string", "description": "New folder name"}
                }),
                &["project_id", "parent_folder_id", "name"],
            ),
        ),
        Tool::new(
            "item_info",
            "Get detailed information about a file/item. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "item_id": {"type": "string", "description": "The item ID"}
                }),
                &["project_id", "item_id"],
            ),
        ),
        Tool::new(
            "item_versions",
            "List all versions of a file/item. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "item_id": {"type": "string", "description": "The item ID"}
                }),
                &["project_id", "item_id"],
            ),
        ),

        // ================================================================
        // Issues Tools
        // ================================================================
        Tool::new(
            "issue_list",
            "List issues in an ACC/BIM360 project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "filter": {"type": "string", "description": "Filter query (e.g., 'status=open')"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "issue_get",
            "Get detailed information about a specific issue.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "issue_id": {"type": "string", "description": "The issue ID"}
                }),
                &["project_id", "issue_id"],
            ),
        ),
        Tool::new(
            "issue_create",
            "Create a new issue in an ACC/BIM360 project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "Issue title"},
                    "description": {"type": "string", "description": "Issue description"},
                    "status": {"type": "string", "description": "Status: open, pending, closed (default: open)"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "issue_update",
            "Update an existing issue.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "issue_id": {"type": "string", "description": "The issue ID"},
                    "title": {"type": "string", "description": "New title"},
                    "description": {"type": "string", "description": "New description"},
                    "status": {"type": "string", "description": "New status: open, pending, closed"}
                }),
                &["project_id", "issue_id"],
            ),
        ),

        // ================================================================
        // RFI Tools
        // ================================================================
        Tool::new(
            "rfi_list",
            "List RFIs (Requests for Information) in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "rfi_get",
            "Get detailed information about a specific RFI.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "rfi_id": {"type": "string", "description": "The RFI ID"}
                }),
                &["project_id", "rfi_id"],
            ),
        ),

        // ================================================================
        // ACC Extended Tools
        // ================================================================
        Tool::new(
            "acc_assets_list",
            "List assets in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "acc_submittals_list",
            "List submittals in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "acc_checklists_list",
            "List checklists in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
    ]
}

// ServerHandler implementation
impl ServerHandler for RapsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "RAPS MCP Server v4.0 - Autodesk Platform Services CLI\n\n\
                Provides direct access to APS APIs (35+ tools):\n\
                * auth_* - Authentication (2-legged and 3-legged OAuth)\n\
                * bucket_*, object_* - OSS storage operations\n\
                * translate_* - CAD model translation\n\
                * hub_*, project_* - Data Management\n\
                * folder_*, item_* - Folder and file management\n\
                * admin_* - Bulk account administration (v4.0)\n\
                * issue_*, rfi_* - ACC Issues and RFIs\n\
                * acc_* - ACC Assets, Submittals, Checklists\n\n\
                Set APS_CLIENT_ID and APS_CLIENT_SECRET env vars.\n\
                For 3-legged auth, run 'raps auth login' first."
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
