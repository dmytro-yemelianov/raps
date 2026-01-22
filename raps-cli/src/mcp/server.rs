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
    AccClient, CreateAssetRequest, CreateChecklistRequest, CreateIssueRequest, CreateRfiRequest,
    CreateSubmittalRequest, IssuesClient, RfiClient, UpdateAssetRequest, UpdateChecklistRequest,
    UpdateIssueRequest, UpdateRfiRequest, UpdateSubmittalRequest, admin::AccountAdminClient,
    permissions::FolderPermissionsClient, users::ProjectUsersClient,
};
use raps_admin::{BulkConfig, FolderType, PermissionLevel, ProjectFilter, StateManager};
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
        IssuesClient::new_with_http_config((*self.config).clone(), auth, self.http_config.clone())
    }

    // Helper to get RFI client (created on demand, not cached)
    async fn get_rfi_client(&self) -> RfiClient {
        let auth = self.get_auth_client().await;
        RfiClient::new_with_http_config((*self.config).clone(), auth, self.http_config.clone())
    }

    // Helper to get ACC Extended client (created on demand, not cached)
    async fn get_acc_client(&self) -> AccClient {
        let auth = self.get_auth_client().await;
        AccClient::new_with_http_config((*self.config).clone(), auth, self.http_config.clone())
    }

    // Helper to get Folder Permissions client (created on demand, not cached)
    async fn get_permissions_client(&self) -> FolderPermissionsClient {
        let auth = self.get_auth_client().await;
        FolderPermissionsClient::new_with_http_config(
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

    async fn auth_login(&self) -> String {
        // MCP cannot perform interactive OAuth login directly
        // Return guidance for the user
        "3-legged OAuth login requires browser interaction.\n\n\
        To authenticate, run one of the following commands in your terminal:\n\n\
        1. Browser-based login (recommended):\n\
           raps auth login\n\n\
        2. Device code flow (for headless environments):\n\
           raps auth login --device\n\n\
        After completing authentication, your MCP session will automatically use the stored tokens."
            .to_string()
    }

    async fn auth_logout(&self) -> String {
        let auth = self.get_auth_client().await;

        match auth.logout().await {
            Ok(()) => "Successfully logged out. 3-legged OAuth tokens have been cleared.".to_string(),
            Err(e) => format!("Logout failed: {}", e),
        }
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

                let mut output = format!(
                    "Found {} project(s) in account {}:\n\n",
                    filtered.len(),
                    account_id
                );
                for proj in &filtered {
                    let status = proj.status.as_deref().unwrap_or("unknown");
                    let platform = if proj.is_acc() {
                        "ACC"
                    } else if proj.is_bim360() {
                        "BIM360"
                    } else {
                        "unknown"
                    };
                    output.push_str(&format!(
                        "* {} (id: {}, status: {}, platform: {})\n",
                        proj.name, proj.id, status, platform
                    ));
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
        )
        .await
        {
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
                            output.push_str(&format!(
                                "  * {}: {}\n",
                                detail.project_name.as_deref().unwrap_or(&detail.project_id),
                                error
                            ));
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
        )
        .await
        {
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
        )
        .await
        {
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

    async fn admin_folder_rights(
        &self,
        account_id: String,
        email: String,
        level: String,
        folder: Option<String>,
        filter: Option<String>,
        dry_run: bool,
    ) -> String {
        // Parse permission level
        let permission_level = match level.to_lowercase().as_str() {
            "view_only" => PermissionLevel::ViewOnly,
            "view_download" => PermissionLevel::ViewDownload,
            "upload_only" => PermissionLevel::UploadOnly,
            "view_download_upload" => PermissionLevel::ViewDownloadUpload,
            "view_download_upload_edit" => PermissionLevel::ViewDownloadUploadEdit,
            "folder_control" => PermissionLevel::FolderControl,
            _ => {
                return format!(
                    "Invalid permission level: '{}'. Valid: view_only, view_download, upload_only, view_download_upload, view_download_upload_edit, folder_control",
                    level
                );
            }
        };

        // Parse folder type
        let folder_type = match folder.as_deref().unwrap_or("project_files") {
            "project_files" => FolderType::ProjectFiles,
            "plans" => FolderType::Plans,
            id => FolderType::Custom(id.to_string()),
        };

        // Parse filter
        let project_filter = if let Some(ref f) = filter {
            match ProjectFilter::from_expression(f) {
                Ok(pf) => pf,
                Err(e) => return format!("Invalid filter expression: {}", e),
            }
        } else {
            ProjectFilter::new()
        };

        let admin_client = self.get_admin_client().await;
        let permissions_client = Arc::new(self.get_permissions_client().await);

        let bulk_config = BulkConfig {
            concurrency: 10,
            dry_run,
            ..Default::default()
        };

        let on_progress = |_| {};

        match raps_admin::bulk_update_folder_rights(
            &admin_client,
            permissions_client,
            &account_id,
            &email,
            permission_level,
            folder_type,
            &project_filter,
            bulk_config,
            on_progress,
        )
        .await
        {
            Ok(result) => {
                format!(
                    "Bulk folder rights operation {}:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s",
                    if dry_run { "(DRY RUN)" } else { "completed" },
                    result.total,
                    result.completed,
                    result.skipped,
                    result.failed,
                    result.duration.as_secs_f64()
                )
            }
            Err(e) => format!("Bulk folder rights failed: {}", e),
        }
    }

    async fn admin_operation_list(&self, limit: Option<usize>) -> String {
        let limit = Self::clamp_limit(limit, 10, 50);

        match StateManager::new() {
            Ok(state_manager) => match state_manager.list_operations(None).await {
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
            },
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
                        let completed = state
                            .results
                            .values()
                            .filter(|r| matches!(r.result, raps_admin::ItemResult::Success))
                            .count();
                        let skipped = state
                            .results
                            .values()
                            .filter(|r| matches!(r.result, raps_admin::ItemResult::Skipped { .. }))
                            .count();
                        let failed = state
                            .results
                            .values()
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

    async fn admin_operation_cancel(&self, operation_id: String) -> String {
        let op_id = match uuid::Uuid::parse_str(&operation_id) {
            Ok(uuid) => uuid,
            Err(_) => return "Invalid operation ID format".to_string(),
        };

        match StateManager::new() {
            Ok(state_manager) => match state_manager.cancel_operation(op_id).await {
                Ok(()) => format!("Operation {} cancelled successfully", operation_id),
                Err(e) => format!("Failed to cancel operation: {}", e),
            },
            Err(e) => format!("Failed to initialize state manager: {}", e),
        }
    }

    async fn admin_operation_resume(&self, operation_id: Option<String>) -> String {
        let state_manager = match StateManager::new() {
            Ok(sm) => sm,
            Err(e) => return format!("Failed to initialize state manager: {}", e),
        };

        // Get the operation ID (from param or most recent)
        let op_id = match operation_id {
            Some(id) => match uuid::Uuid::parse_str(&id) {
                Ok(uuid) => uuid,
                Err(_) => return "Invalid operation ID format".to_string(),
            },
            None => {
                // Get most recent incomplete operation
                match state_manager.list_operations(None).await {
                    Ok(ops) => {
                        match ops
                            .iter()
                            .find(|o| o.status == raps_admin::OperationStatus::InProgress)
                        {
                            Some(op) => op.operation_id,
                            None => return "No in-progress operations to resume".to_string(),
                        }
                    }
                    Err(e) => return format!("Failed to list operations: {}", e),
                }
            }
        };

        // Load the operation state
        let state = match state_manager.load_operation(op_id).await {
            Ok(s) => s,
            Err(e) => return format!("Failed to load operation: {}", e),
        };

        // Check operation status
        if state.status != raps_admin::OperationStatus::InProgress {
            return format!(
                "Operation {} has status {:?} and cannot be resumed",
                op_id, state.status
            );
        }

        let bulk_config = BulkConfig {
            concurrency: 10,
            dry_run: false,
            ..Default::default()
        };
        let on_progress = |_| {};

        // Resume based on operation type
        match state.operation_type {
            raps_admin::OperationType::AddUser => {
                let users_client = Arc::new(self.get_users_client().await);
                match raps_admin::resume_bulk_add_user(
                    users_client,
                    op_id,
                    bulk_config,
                    on_progress,
                )
                .await
                {
                    Ok(result) => format!(
                        "Resumed add user operation:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s",
                        result.total,
                        result.completed,
                        result.skipped,
                        result.failed,
                        result.duration.as_secs_f64()
                    ),
                    Err(e) => format!("Failed to resume operation: {}", e),
                }
            }
            raps_admin::OperationType::RemoveUser => {
                let users_client = Arc::new(self.get_users_client().await);
                match raps_admin::resume_bulk_remove_user(
                    users_client,
                    op_id,
                    bulk_config,
                    on_progress,
                )
                .await
                {
                    Ok(result) => format!(
                        "Resumed remove user operation:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s",
                        result.total,
                        result.completed,
                        result.skipped,
                        result.failed,
                        result.duration.as_secs_f64()
                    ),
                    Err(e) => format!("Failed to resume operation: {}", e),
                }
            }
            raps_admin::OperationType::UpdateRole => {
                let users_client = Arc::new(self.get_users_client().await);
                match raps_admin::resume_bulk_update_role(
                    users_client,
                    op_id,
                    bulk_config,
                    on_progress,
                )
                .await
                {
                    Ok(result) => format!(
                        "Resumed update role operation:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s",
                        result.total,
                        result.completed,
                        result.skipped,
                        result.failed,
                        result.duration.as_secs_f64()
                    ),
                    Err(e) => format!("Failed to resume operation: {}", e),
                }
            }
            raps_admin::OperationType::UpdateFolderRights => {
                let permissions_client = Arc::new(self.get_permissions_client().await);
                match raps_admin::resume_bulk_update_folder_rights(
                    permissions_client,
                    op_id,
                    bulk_config,
                    on_progress,
                )
                .await
                {
                    Ok(result) => format!(
                        "Resumed folder rights operation:\n\n* Total: {}\n* Completed: {}\n* Skipped: {}\n* Failed: {}\n* Duration: {:.2}s",
                        result.total,
                        result.completed,
                        result.skipped,
                        result.failed,
                        result.duration.as_secs_f64()
                    ),
                    Err(e) => format!("Failed to resume operation: {}", e),
                }
            }
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
                    let item_type = item
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");
                    let name = item
                        .get("attributes")
                        .and_then(|a| a.get("displayName").or(a.get("name")))
                        .and_then(|n| n.as_str())
                        .unwrap_or("Unnamed");
                    let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("unknown");
                    let icon = if item_type == "folders" {
                        "[folder]"
                    } else {
                        "[file]"
                    };
                    output.push_str(&format!("* {} {} (id: {})\n", icon, name, id));
                }
                output
            }
            Err(e) => format!("Failed to list folder contents: {}", e),
        }
    }

    async fn folder_create(
        &self,
        project_id: String,
        parent_folder_id: String,
        name: String,
    ) -> String {
        let client = self.get_dm_client().await;

        match client
            .create_folder(&project_id, &parent_folder_id, &name)
            .await
        {
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
                    item.attributes.display_name, item.id, item.item_type
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
                    let ver_num = v
                        .attributes
                        .version_number
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let name = v
                        .attributes
                        .display_name
                        .as_ref()
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
                    let display_id = issue
                        .display_id
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "-".to_string());
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

    #[allow(clippy::too_many_arguments)]
    async fn rfi_create(
        &self,
        project_id: String,
        title: String,
        question: Option<String>,
        priority: Option<String>,
        due_date: Option<String>,
        assigned_to: Option<String>,
        location: Option<String>,
    ) -> String {
        let client = self.get_rfi_client().await;

        let request = CreateRfiRequest {
            title,
            question,
            priority,
            due_date,
            assigned_to,
            location,
            discipline: None,
        };

        match client.create_rfi(&project_id, request).await {
            Ok(rfi) => {
                format!(
                    "RFI created successfully:\n\n* ID: {}\n* Title: {}\n* Status: {}",
                    rfi.id, rfi.title, rfi.status
                )
            }
            Err(e) => format!("Failed to create RFI: {}", e),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn rfi_update(
        &self,
        project_id: String,
        rfi_id: String,
        title: Option<String>,
        question: Option<String>,
        answer: Option<String>,
        status: Option<String>,
        priority: Option<String>,
    ) -> String {
        let client = self.get_rfi_client().await;

        let request = UpdateRfiRequest {
            title,
            question,
            answer,
            status,
            priority,
            due_date: None,
            assigned_to: None,
            location: None,
        };

        match client.update_rfi(&project_id, &rfi_id, request).await {
            Ok(rfi) => {
                format!(
                    "RFI updated successfully:\n\n* ID: {}\n* Title: {}\n* Status: {}",
                    rfi.id, rfi.title, rfi.status
                )
            }
            Err(e) => format!("Failed to update RFI: {}", e),
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

    async fn asset_create(
        &self,
        project_id: String,
        category_id: Option<String>,
        description: Option<String>,
        barcode: Option<String>,
        client_asset_id: Option<String>,
    ) -> String {
        let client = self.get_acc_client().await;

        let request = CreateAssetRequest {
            category_id,
            description,
            barcode,
            client_asset_id,
        };

        match client.create_asset(&project_id, request).await {
            Ok(asset) => {
                format!(
                    "Asset created successfully:\n\n* ID: {}\n* Description: {}",
                    asset.id,
                    asset.description.as_deref().unwrap_or("-")
                )
            }
            Err(e) => format!("Failed to create asset: {}", e),
        }
    }

    async fn asset_update(
        &self,
        project_id: String,
        asset_id: String,
        category_id: Option<String>,
        status_id: Option<String>,
        description: Option<String>,
        barcode: Option<String>,
    ) -> String {
        let client = self.get_acc_client().await;

        let request = UpdateAssetRequest {
            category_id,
            status_id,
            description,
            barcode,
        };

        match client.update_asset(&project_id, &asset_id, request).await {
            Ok(asset) => {
                format!(
                    "Asset updated successfully:\n\n* ID: {}\n* Description: {}",
                    asset.id,
                    asset.description.as_deref().unwrap_or("-")
                )
            }
            Err(e) => format!("Failed to update asset: {}", e),
        }
    }

    async fn asset_delete(&self, project_id: String, asset_id: String) -> String {
        let client = self.get_acc_client().await;

        match client.delete_asset(&project_id, &asset_id).await {
            Ok(()) => format!("Asset {} deleted successfully", asset_id),
            Err(e) => format!("Failed to delete asset: {}", e),
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

    async fn submittal_create(
        &self,
        project_id: String,
        title: String,
        spec_section: Option<String>,
        due_date: Option<String>,
    ) -> String {
        let client = self.get_acc_client().await;

        let request = CreateSubmittalRequest {
            title,
            spec_section,
            due_date,
        };

        match client.create_submittal(&project_id, request).await {
            Ok(submittal) => {
                format!(
                    "Submittal created successfully:\n\n* ID: {}\n* Title: {}\n* Status: {}",
                    submittal.id, submittal.title, submittal.status
                )
            }
            Err(e) => format!("Failed to create submittal: {}", e),
        }
    }

    async fn submittal_update(
        &self,
        project_id: String,
        submittal_id: String,
        title: Option<String>,
        status: Option<String>,
        due_date: Option<String>,
    ) -> String {
        let client = self.get_acc_client().await;

        let request = UpdateSubmittalRequest {
            title,
            status,
            due_date,
        };

        match client
            .update_submittal(&project_id, &submittal_id, request)
            .await
        {
            Ok(submittal) => {
                format!(
                    "Submittal updated successfully:\n\n* ID: {}\n* Title: {}\n* Status: {}",
                    submittal.id, submittal.title, submittal.status
                )
            }
            Err(e) => format!("Failed to update submittal: {}", e),
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

    async fn checklist_create(
        &self,
        project_id: String,
        title: String,
        template_id: Option<String>,
        location: Option<String>,
        due_date: Option<String>,
        assignee_id: Option<String>,
    ) -> String {
        let client = self.get_acc_client().await;

        let request = CreateChecklistRequest {
            title,
            template_id,
            location,
            due_date,
            assignee_id,
        };

        match client.create_checklist(&project_id, request).await {
            Ok(checklist) => {
                format!(
                    "Checklist created successfully:\n\n* ID: {}\n* Title: {}\n* Status: {}",
                    checklist.id, checklist.title, checklist.status
                )
            }
            Err(e) => format!("Failed to create checklist: {}", e),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn checklist_update(
        &self,
        project_id: String,
        checklist_id: String,
        title: Option<String>,
        status: Option<String>,
        location: Option<String>,
        due_date: Option<String>,
        assignee_id: Option<String>,
    ) -> String {
        let client = self.get_acc_client().await;

        let request = UpdateChecklistRequest {
            title,
            status,
            location,
            due_date,
            assignee_id,
        };

        match client
            .update_checklist(&project_id, &checklist_id, request)
            .await
        {
            Ok(checklist) => {
                format!(
                    "Checklist updated successfully:\n\n* ID: {}\n* Title: {}\n* Status: {}",
                    checklist.id, checklist.title, checklist.status
                )
            }
            Err(e) => format!("Failed to update checklist: {}", e),
        }
    }

    // ========================================================================
    // Object Upload/Download Operations (v4.4)
    // ========================================================================

    async fn object_upload(
        &self,
        bucket_key: String,
        file_path: String,
        object_key: Option<String>,
    ) -> String {
        use std::path::Path;

        let path = Path::new(&file_path);

        // Validate file exists
        if !path.exists() {
            return format!("Error: File not found: {}", file_path);
        }

        if !path.is_file() {
            return format!("Error: Path is not a file: {}", file_path);
        }

        // Determine object key from filename if not provided
        let obj_key = object_key.unwrap_or_else(|| {
            path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".to_string())
        });

        let client = self.get_oss_client().await;

        match client.upload_object(&bucket_key, &obj_key, path).await {
            Ok(info) => {
                let urn = client.get_urn(&bucket_key, &obj_key);
                format!(
                    "Uploaded '{}' to '{}'\n* Object Key: {}\n* Size: {} bytes\n* SHA1: {}\n* URN: {}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    bucket_key,
                    info.object_key,
                    info.size,
                    info.sha1.unwrap_or_else(|| "-".to_string()),
                    urn
                )
            }
            Err(e) => format!("Failed to upload file: {}", e),
        }
    }

    async fn object_upload_batch(&self, bucket_key: String, file_paths: Vec<String>) -> String {
        use std::path::Path;
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        if file_paths.is_empty() {
            return "Error: No files specified for upload.".to_string();
        }

        let client = self.get_oss_client().await;
        let semaphore = Arc::new(Semaphore::new(4)); // 4-way concurrency

        let mut handles = Vec::new();

        for file_path in file_paths.clone() {
            let client = client.clone();
            let bucket_key = bucket_key.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit until done
                let path = std::path::PathBuf::from(&file_path);

                if !path.exists() {
                    return (
                        file_path,
                        false,
                        None::<u64>,
                        Some("File not found".to_string()),
                    );
                }

                if !path.is_file() {
                    return (
                        file_path,
                        false,
                        None::<u64>,
                        Some("Not a file".to_string()),
                    );
                }

                let obj_key = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unnamed".to_string());

                match client.upload_object(&bucket_key, &obj_key, &path).await {
                    Ok(info) => (file_path, true, Some(info.size), None::<String>),
                    Err(e) => (file_path, false, None::<u64>, Some(e.to_string())),
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut successful = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        for handle in handles {
            if let Ok((path, success, size, error)) = handle.await {
                let path_display = Path::new(&path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or(path);

                if success {
                    successful += 1;
                    let size_display = size.map(format_size).unwrap_or_default();
                    results.push(format!("✓ {} ({})", path_display, size_display));
                } else {
                    failed += 1;
                    let err_msg = error.unwrap_or_else(|| "Unknown error".to_string());
                    results.push(format!("✗ {} ({})", path_display, err_msg));
                }
            }
        }

        format!(
            "Batch upload complete: {} succeeded, {} failed\n\nResults:\n{}",
            successful,
            failed,
            results.join("\n")
        )
    }

    async fn object_download(
        &self,
        bucket_key: String,
        object_key: String,
        output_path: String,
    ) -> String {
        use std::path::Path;

        let client = self.get_oss_client().await;

        // Check if parent directory exists
        let path = Path::new(&output_path);
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            return format!("Error: Directory does not exist: {}", parent.display());
        }

        match client.download_object(&bucket_key, &object_key, path).await {
            Ok(()) => {
                // Get file size from downloaded file
                let size = std::fs::metadata(&output_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                format!(
                    "Downloaded '{}' to '{}'\n* Size: {} bytes",
                    object_key, output_path, size
                )
            }
            Err(e) => format!("Failed to download object: {}", e),
        }
    }

    async fn object_info(&self, bucket_key: String, object_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.get_object_details(&bucket_key, &object_key).await {
            Ok(details) => {
                let size_display = format_size(details.size);
                let urn = client.get_urn(&bucket_key, &object_key);

                format!(
                    "Object: {} in {}\n\n\
                     * Size: {} bytes ({})\n\
                     * Content-Type: {}\n\
                     * SHA1: {}\n\
                     * Created: {}\n\
                     * Modified: {}\n\
                     * URN: {}",
                    details.object_key,
                    details.bucket_key,
                    details.size,
                    size_display,
                    details.content_type,
                    details.sha1,
                    details.created_date.unwrap_or_else(|| "-".to_string()),
                    details
                        .last_modified_date
                        .unwrap_or_else(|| "-".to_string()),
                    urn
                )
            }
            Err(e) => format!("Failed to get object details: {}", e),
        }
    }

    async fn object_copy(
        &self,
        source_bucket: String,
        source_key: String,
        dest_bucket: String,
        dest_key: Option<String>,
    ) -> String {
        let client = self.get_oss_client().await;
        let destination_key = dest_key.unwrap_or_else(|| source_key.clone());

        // Check if destination exists first (non-destructive)
        match client
            .get_object_details(&dest_bucket, &destination_key)
            .await
        {
            Ok(existing) => {
                return format!(
                    "Warning: Object '{}' already exists in '{}' (skipped)\n\
                     * Existing size: {} bytes\n\
                     * Delete it first if you want to overwrite.",
                    destination_key, dest_bucket, existing.size
                );
            }
            Err(_) => {
                // Object doesn't exist, proceed with copy
            }
        }

        // OSS doesn't have a direct copy API, so we download and re-upload
        // For now, use a temporary file approach
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("raps_copy_{}", uuid::Uuid::new_v4()));

        // Download to temp
        match client
            .download_object(&source_bucket, &source_key, &temp_path)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                return format!("Failed to read source object: {}", e);
            }
        }

        // Upload to destination
        let result = match client
            .upload_object(&dest_bucket, &destination_key, &temp_path)
            .await
        {
            Ok(info) => {
                let urn = client.get_urn(&dest_bucket, &destination_key);
                format!(
                    "Copied '{}' from '{}' to '{}'\n* Size: {} bytes\n* New URN: {}",
                    source_key, source_bucket, dest_bucket, info.size, urn
                )
            }
            Err(e) => format!("Failed to copy to destination: {}", e),
        };

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        result
    }

    async fn object_delete_batch(&self, bucket_key: String, object_keys: Vec<String>) -> String {
        if object_keys.is_empty() {
            return "Error: No objects specified for deletion.".to_string();
        }

        let client = self.get_oss_client().await;
        let mut deleted = 0;
        let mut skipped = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        for object_key in &object_keys {
            match client.delete_object(&bucket_key, object_key).await {
                Ok(()) => {
                    deleted += 1;
                    results.push(format!("✓ {} (deleted)", object_key));
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("404") || err_str.contains("not found") {
                        skipped += 1;
                        results.push(format!("○ {} (not found, skipped)", object_key));
                    } else {
                        failed += 1;
                        results.push(format!("✗ {} ({})", object_key, err_str));
                    }
                }
            }
        }

        format!(
            "Batch delete complete: {} deleted, {} skipped, {} failed\n\nResults:\n{}",
            deleted,
            skipped,
            failed,
            results.join("\n")
        )
    }

    // ========================================================================
    // Project Management Operations (v4.4)
    // ========================================================================

    async fn project_info(&self, hub_id: String, project_id: String) -> String {
        let client = self.get_dm_client().await;

        match client.get_project(&hub_id, &project_id).await {
            Ok(project) => {
                let mut output = format!(
                    "Project: {}\n* ID: {}\n* Hub: {}\n* Scopes: {}\n",
                    project.attributes.name,
                    project.id,
                    hub_id,
                    project
                        .attributes
                        .scopes
                        .as_ref()
                        .map(|s| s.join(", "))
                        .unwrap_or_else(|| "-".to_string())
                );

                // Get top folders
                if let Ok(folders) = client.get_top_folders(&hub_id, &project_id).await
                    && !folders.is_empty()
                {
                    output.push_str("\nTop Folders:\n");
                    for folder in &folders {
                        let display_name = folder
                            .attributes
                            .display_name
                            .as_ref()
                            .unwrap_or(&folder.attributes.name);
                        output.push_str(&format!("* {} ({})\n", display_name, folder.id));
                    }
                }

                output
            }
            Err(e) => format!("Failed to get project: {}", e),
        }
    }

    async fn project_users_list(
        &self,
        project_id: String,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> String {
        let client = self.get_users_client().await;
        let limit = limit.unwrap_or(50).min(200);

        match client
            .list_project_users(&project_id, Some(limit), offset)
            .await
        {
            Ok(response) => {
                if response.results.is_empty() {
                    return "No users found in project.".to_string();
                }

                let start = offset.unwrap_or(0) + 1;
                let end = start + response.results.len() - 1;
                let total = response.pagination.total_results;

                let mut output = format!(
                    "Project Users (showing {}-{} of {}):\n\n",
                    start, end, total
                );

                for (i, user) in response.results.iter().enumerate() {
                    let name = user.name.as_deref().unwrap_or("-");
                    let email = user.email.as_deref().unwrap_or("-");
                    let role = user
                        .role_name
                        .as_deref()
                        .unwrap_or(user.role_id.as_deref().unwrap_or("-"));

                    output.push_str(&format!(
                        "{}. {} ({})\n   * Role: {}\n\n",
                        start + i,
                        name,
                        email,
                        role
                    ));
                }

                if response.has_more() {
                    output.push_str(&format!(
                        "Use offset={} to see next page.",
                        response.next_offset()
                    ));
                }

                output
            }
            Err(e) => format!("Failed to list project users: {}", e),
        }
    }

    async fn folder_contents(
        &self,
        project_id: String,
        folder_id: String,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> String {
        let client = self.get_dm_client().await;

        match client.list_folder_contents(&project_id, &folder_id).await {
            Ok(contents) => {
                if contents.is_empty() {
                    return "Folder is empty.".to_string();
                }

                let offset_val = offset.unwrap_or(0);
                let limit_val = limit.unwrap_or(50);
                let total = contents.len();

                let items: Vec<_> = contents
                    .into_iter()
                    .skip(offset_val)
                    .take(limit_val)
                    .collect();

                let start = offset_val + 1;
                let end = offset_val + items.len();

                let mut output = format!(
                    "Folder Contents (showing {}-{} of {}):\n\n",
                    start, end, total
                );

                let mut folders = Vec::new();
                let mut files = Vec::new();

                for item in items {
                    let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                    let name = item
                        .get("attributes")
                        .and_then(|a| a.get("displayName").or_else(|| a.get("name")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");

                    if item_type == "folders" {
                        folders.push(format!("📁 {} ({})", name, id));
                    } else {
                        files.push(format!("📄 {}", name));
                    }
                }

                if !folders.is_empty() {
                    output.push_str("Subfolders:\n");
                    for f in &folders {
                        output.push_str(&format!("{}\n", f));
                    }
                    output.push('\n');
                }

                if !files.is_empty() {
                    output.push_str("Items:\n");
                    for f in &files {
                        output.push_str(&format!("{}\n", f));
                    }
                }

                if end < total {
                    output.push_str(&format!("\nUse offset={} to see next page.", end));
                }

                output
            }
            Err(e) => format!("Failed to list folder contents: {}", e),
        }
    }

    // ========================================================================
    // ACC Project Admin Operations (v4.4)
    // ========================================================================

    async fn project_create(
        &self,
        account_id: String,
        name: String,
        template_project_id: Option<String>,
        products: Option<Vec<String>>,
    ) -> String {
        use raps_acc::CreateProjectRequest;

        let client = self.get_acc_client().await;

        let request = CreateProjectRequest {
            name: name.clone(),
            template_project_id,
            products,
            project_type: Some("ACC".to_string()),
        };

        match client.create_project(&account_id, request).await {
            Ok(job) => {
                let project_id = match job.project_id {
                    Some(id) => id,
                    None => return "Project creation initiated but no ID returned.".to_string(),
                };

                // Wait for activation (up to 60 seconds)
                match client
                    .wait_for_project_activation(&account_id, &project_id, Some(60), Some(2000))
                    .await
                {
                    Ok(final_job) => {
                        let mut output = format!(
                            "Created ACC project: {}\n* ID: {}\n* Account: {}\n* Status: Active",
                            name,
                            final_job.project_id.unwrap_or(project_id),
                            account_id
                        );

                        // Add warning about template members
                        output.push_str(
                            "\n\nNote: Template members are NOT auto-assigned when cloning.",
                        );

                        output
                    }
                    Err(e) => format!(
                        "Project created (ID: {}) but activation timed out: {}\nCheck status later.",
                        project_id, e
                    ),
                }
            }
            Err(e) => format!("Failed to create project: {}", e),
        }
    }

    async fn project_user_add(
        &self,
        project_id: String,
        email: String,
        role_id: Option<String>,
    ) -> String {
        use raps_acc::users::AddProjectUserRequest;

        let client = self.get_users_client().await;

        // Note: The API expects user_id, not email. The MCP tool description should clarify this.
        // In practice, the caller should look up the user ID from email first.
        let request = AddProjectUserRequest {
            user_id: email.clone(), // Using email as placeholder - caller should resolve to user_id
            role_id,
            products: vec![],
        };

        match client.add_user(&project_id, request).await {
            Ok(user) => format!(
                "Added user to project:\n* Email: {}\n* Name: {}\n* Role: {}",
                email,
                user.name.unwrap_or_else(|| "-".to_string()),
                user.role_name
                    .unwrap_or_else(|| user.role_id.unwrap_or_else(|| "-".to_string()))
            ),
            Err(e) => format!("Failed to add user to project: {}", e),
        }
    }

    async fn project_users_import(&self, project_id: String, users: Vec<Value>) -> String {
        use raps_acc::users::ImportUserRequest;

        let client = self.get_users_client().await;

        // Parse user objects from JSON
        let import_requests: Vec<ImportUserRequest> = users
            .into_iter()
            .filter_map(|v| {
                let email = v.get("email")?.as_str()?.to_string();
                let role_id = v.get("role_id").and_then(|r| r.as_str()).map(String::from);
                Some(ImportUserRequest {
                    email,
                    role_id,
                    products: None,
                })
            })
            .collect();

        if import_requests.is_empty() {
            return "Error: No valid users provided for import.".to_string();
        }

        match client.import_users(&project_id, import_requests).await {
            Ok(result) => {
                let mut output = format!(
                    "User import complete: {} imported, {} failed\n\nResults:\n",
                    result.imported, result.failed
                );

                for success in &result.successes {
                    output.push_str(&format!("✓ {}\n", success.email));
                }

                for error in &result.errors {
                    output.push_str(&format!("✗ {} ({})\n", error.email, error.error));
                }

                output
            }
            Err(e) => format!("Failed to import users: {}", e),
        }
    }

    async fn project_update(
        &self,
        account_id: String,
        project_id: String,
        name: Option<String>,
        status: Option<String>,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> String {
        use raps_acc::admin::UpdateProjectRequest;

        let client = self.get_admin_client().await;

        if name.is_none() && status.is_none() && start_date.is_none() && end_date.is_none() {
            return "Error: At least one field to update must be provided.".to_string();
        }

        let request = UpdateProjectRequest {
            name: name.clone(),
            status: status.clone(),
            start_date,
            end_date,
            ..Default::default()
        };

        match client.update_project(&account_id, &project_id, request).await {
            Ok(project) => format!(
                "Updated project:\n* Name: {}\n* ID: {}\n* Status: {}",
                project.name,
                project.id,
                project.status.unwrap_or_else(|| "unknown".to_string())
            ),
            Err(e) => format!("Failed to update project: {}", e),
        }
    }

    async fn project_archive(&self, account_id: String, project_id: String) -> String {
        let client = self.get_admin_client().await;

        // Get project name before archiving
        let project_name = match client.get_project(&account_id, &project_id).await {
            Ok(project) => project.name,
            Err(_) => "-".to_string(),
        };

        match client.archive_project(&account_id, &project_id).await {
            Ok(()) => format!(
                "Archived project:\n* Name: {}\n* ID: {}\n* Status: archived",
                project_name, project_id
            ),
            Err(e) => format!("Failed to archive project: {}", e),
        }
    }

    async fn project_user_remove(&self, project_id: String, user_id: String) -> String {
        let client = self.get_users_client().await;

        match client.remove_user(&project_id, &user_id).await {
            Ok(()) => format!(
                "Removed user from project:\n* User ID: {}\n* Project ID: {}",
                user_id, project_id
            ),
            Err(e) => format!("Failed to remove user from project: {}", e),
        }
    }

    async fn project_user_update(
        &self,
        project_id: String,
        user_id: String,
        role_id: Option<String>,
    ) -> String {
        use raps_acc::users::UpdateProjectUserRequest;

        let client = self.get_users_client().await;

        if role_id.is_none() {
            return "Error: At least role_id must be provided.".to_string();
        }

        let request = UpdateProjectUserRequest {
            role_id,
            products: None,
        };

        match client.update_user(&project_id, &user_id, request).await {
            Ok(user) => format!(
                "Updated user in project:\n* User ID: {}\n* Name: {}\n* Role: {}",
                user.id,
                user.name.unwrap_or_else(|| "-".to_string()),
                user.role_name
                    .unwrap_or_else(|| user.role_id.unwrap_or_else(|| "-".to_string()))
            ),
            Err(e) => format!("Failed to update user in project: {}", e),
        }
    }

    // ========================================================================
    // Template Management Operations (v4.5)
    // ========================================================================

    async fn template_list(&self, account_id: String, limit: Option<usize>) -> String {
        let client = self.get_admin_client().await;

        match client
            .list_templates(&account_id, limit, None)
            .await
        {
            Ok(response) => {
                if response.results.is_empty() {
                    return "No templates found in this account.".to_string();
                }

                let mut output = format!(
                    "Templates in account {} ({} total):\n\n",
                    account_id, response.pagination.total_results
                );

                for template in &response.results {
                    let status = template.status.as_deref().unwrap_or("unknown");
                    output.push_str(&format!(
                        "* {} (ID: {})\n  Status: {} | Platform: {}\n",
                        template.name,
                        template.id,
                        status,
                        template.platform.as_deref().unwrap_or("unknown")
                    ));
                }

                output
            }
            Err(e) => format!("Failed to list templates: {}", e),
        }
    }

    async fn template_info(&self, account_id: String, template_id: String) -> String {
        let client = self.get_admin_client().await;

        match client.get_project(&account_id, &template_id).await {
            Ok(project) => {
                if !project.is_template() {
                    return format!(
                        "Project {} is not a template (classification: {:?})",
                        template_id, project.classification
                    );
                }

                let mut output = format!(
                    "Template: {}\n\n\
                    * ID: {}\n\
                    * Status: {}\n\
                    * Platform: {}\n\
                    * Classification: template\n",
                    project.name,
                    project.id,
                    project.status.as_deref().unwrap_or("unknown"),
                    project.platform.as_deref().unwrap_or("unknown"),
                );

                if let Some(members) = project.member_count {
                    output.push_str(&format!("* Members: {}\n", members));
                }

                if let Some(companies) = project.company_count {
                    output.push_str(&format!("* Companies: {}\n", companies));
                }

                let products = project.enabled_products();
                if !products.is_empty() {
                    output.push_str(&format!("* Products: {:?}\n", products));
                }

                output
            }
            Err(e) => format!("Failed to get template: {}", e),
        }
    }

    async fn template_create(
        &self,
        account_id: String,
        name: String,
        products: Option<Vec<String>>,
    ) -> String {
        use raps_acc::admin::CreateProjectRequest;
        use raps_acc::types::ProjectClassification;

        let client = self.get_admin_client().await;

        let request = CreateProjectRequest {
            name: name.clone(),
            classification: Some(ProjectClassification::Template),
            products,
            ..Default::default()
        };

        match client.create_project(&account_id, request).await {
            Ok(project) => {
                let project_id = project.id.clone();

                // Wait for activation (up to 60 seconds)
                match client
                    .wait_for_project_active(&account_id, &project_id, Some(60), Some(2000))
                    .await
                {
                    Ok(final_project) => format!(
                        "Created template: {}\n* ID: {}\n* Account: {}\n* Status: {}",
                        name,
                        final_project.id,
                        account_id,
                        final_project.status.unwrap_or_else(|| "active".to_string())
                    ),
                    Err(e) => format!(
                        "Template created (ID: {}) but activation timed out: {}\nCheck status later.",
                        project_id, e
                    ),
                }
            }
            Err(e) => format!("Failed to create template: {}", e),
        }
    }

    async fn template_update(
        &self,
        account_id: String,
        template_id: String,
        name: Option<String>,
        status: Option<String>,
    ) -> String {
        use raps_acc::admin::UpdateProjectRequest;

        let client = self.get_admin_client().await;

        // Verify it's a template first
        match client.get_project(&account_id, &template_id).await {
            Ok(project) => {
                if !project.is_template() {
                    return format!(
                        "Project {} is not a template. Use project_update for regular projects.",
                        template_id
                    );
                }
            }
            Err(e) => return format!("Failed to get template: {}", e),
        }

        let request = UpdateProjectRequest {
            name,
            status,
            ..Default::default()
        };

        match client.update_project(&account_id, &template_id, request).await {
            Ok(project) => format!(
                "Updated template:\n* Name: {}\n* ID: {}\n* Status: {}",
                project.name,
                project.id,
                project.status.unwrap_or_else(|| "unknown".to_string())
            ),
            Err(e) => format!("Failed to update template: {}", e),
        }
    }

    async fn template_archive(&self, account_id: String, template_id: String) -> String {
        let client = self.get_admin_client().await;

        // Verify it's a template first
        let template_name = match client.get_project(&account_id, &template_id).await {
            Ok(project) => {
                if !project.is_template() {
                    return format!(
                        "Project {} is not a template. Use project archive for regular projects.",
                        template_id
                    );
                }
                project.name
            }
            Err(e) => return format!("Failed to get template: {}", e),
        };

        match client.archive_project(&account_id, &template_id).await {
            Ok(()) => format!(
                "Archived template:\n* Name: {}\n* ID: {}\n* Status: archived",
                template_name, template_id
            ),
            Err(e) => format!("Failed to archive template: {}", e),
        }
    }

    async fn template_convert(&self, account_id: String, project_id: String) -> String {
        let client = self.get_admin_client().await;

        // Check if project exists and is not already a template
        match client.get_project(&account_id, &project_id).await {
            Ok(project) => {
                if project.is_template() {
                    return format!("Project {} is already a template.", project_id);
                }

                // ACC API does not support changing classification directly
                format!(
                    "Converting existing projects to templates is not supported by the ACC API.\n\n\
                    Project '{}' (ID: {}) cannot be converted.\n\n\
                    Workaround: Create a new template using template_create and configure it manually.",
                    project.name, project_id
                )
            }
            Err(e) => format!("Failed to get project: {}", e),
        }
    }

    // ========================================================================
    // Item Management Operations (v4.4)
    // ========================================================================

    async fn item_create(
        &self,
        project_id: String,
        folder_id: String,
        display_name: String,
        storage_id: String,
    ) -> String {
        let client = self.get_dm_client().await;

        match client
            .create_item_from_storage(&project_id, &folder_id, &display_name, &storage_id)
            .await
        {
            Ok(item) => format!(
                "Created item in project folder:\n* Display Name: {}\n* Item ID: {}\n* Version: 1",
                item.attributes.display_name, item.id
            ),
            Err(e) => format!("Failed to create item: {}", e),
        }
    }

    async fn item_delete(&self, project_id: String, item_id: String) -> String {
        let client = self.get_dm_client().await;

        match client.delete_item(&project_id, &item_id).await {
            Ok(()) => format!("Deleted item from project:\n* Item ID: {}", item_id),
            Err(e) => format!("Failed to delete item: {}", e),
        }
    }

    async fn item_rename(&self, project_id: String, item_id: String, new_name: String) -> String {
        let client = self.get_dm_client().await;

        // Get current item to show old name
        let old_name = match client.get_item(&project_id, &item_id).await {
            Ok(item) => item.attributes.display_name,
            Err(_) => "-".to_string(),
        };

        match client.rename_item(&project_id, &item_id, &new_name).await {
            Ok(item) => format!(
                "Renamed item:\n* Old Name: {}\n* New Name: {}\n* Item ID: {}",
                old_name, item.attributes.display_name, item.id
            ),
            Err(e) => format!("Failed to rename item: {}", e),
        }
    }

    // ================================================================
    // Custom API Requests
    // ================================================================

    async fn api_request(
        &self,
        method: String,
        endpoint: String,
        query: Option<Map<String, Value>>,
        headers: Option<Map<String, Value>>,
        body: Option<Value>,
    ) -> String {
        use reqwest::header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
        use raps_kernel::http::is_allowed_url;

        // Validate HTTP method
        let http_method = match method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            "DELETE" => reqwest::Method::DELETE,
            _ => {
                return format!(
                    "Invalid HTTP method '{}'. Supported: GET, POST, PUT, PATCH, DELETE",
                    method
                );
            }
        };

        // Build full URL from endpoint and query parameters
        let full_url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.clone()
        } else {
            let endpoint = if endpoint.starts_with('/') {
                endpoint.clone()
            } else {
                format!("/{}", endpoint)
            };
            format!("{}{}", self.config.base_url.trim_end_matches('/'), endpoint)
        };

        // Add query parameters
        let full_url = if let Some(query_params) = &query {
            let query_string: String = query_params
                .iter()
                .map(|(k, v)| {
                    let val = v.as_str().map(String::from).unwrap_or_else(|| v.to_string());
                    format!(
                        "{}={}",
                        urlencoding::encode(k),
                        urlencoding::encode(&val)
                    )
                })
                .collect::<Vec<_>>()
                .join("&");
            if full_url.contains('?') {
                format!("{}&{}", full_url, query_string)
            } else {
                format!("{}?{}", full_url, query_string)
            }
        } else {
            full_url
        };

        // Validate URL is allowed (APS domains only)
        if !is_allowed_url(&full_url) {
            return format!(
                "URL not allowed: {}\n\n\
                 Only APS API endpoints are permitted for security reasons.\n\
                 Allowed domains: developer.api.autodesk.com, api.userprofile.autodesk.com, \
                 acc.autodesk.com, developer.autodesk.com, b360dm.autodesk.com, cdn.derivative.autodesk.io",
                full_url
            );
        }

        // Validate body is only used with appropriate methods
        let supports_body = matches!(
            http_method,
            reqwest::Method::POST | reqwest::Method::PUT | reqwest::Method::PATCH
        );
        if body.is_some() && !supports_body {
            return format!(
                "Request body is not allowed for {} requests",
                http_method.as_str()
            );
        }

        // Get auth token
        let auth_client = self.get_auth_client().await;
        let token = match auth_client.get_3leg_token().await {
            Ok(token) => token,
            Err(_) => match auth_client.get_token().await {
                Ok(token) => token,
                Err(e) => {
                    return format!(
                        "Authentication failed: {}\n\n\
                         Run 'raps auth login' for 3-legged auth or configure client credentials.",
                        e
                    );
                }
            },
        };

        // Build HTTP client
        let client = match self.http_config.create_client() {
            Ok(c) => c,
            Err(e) => return format!("Failed to create HTTP client: {}", e),
        };

        // Build request
        let mut request = client.request(http_method.clone(), &full_url);

        // Add authorization
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));

        // Add custom headers (excluding Authorization)
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                if key.to_lowercase() == "authorization" {
                    continue; // Cannot override Authorization header
                }
                let val_str = value
                    .as_str()
                    .map(String::from)
                    .unwrap_or_else(|| value.to_string());
                if let (Ok(name), Ok(val)) = (
                    HeaderName::try_from(key.as_str()),
                    HeaderValue::try_from(&val_str),
                ) {
                    request = request.header(name, val);
                }
            }
        }

        // Add body if present
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, "application/json").json(&body);
        }

        // Execute request
        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => return format!("Request failed: {}", e),
        };

        let status = response.status();
        let status_code = status.as_u16();

        // Collect response headers for display
        let response_headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .take(10) // Limit headers shown
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Get content type
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        // Read response body
        let body_result = response.text().await;
        let body_text = match body_result {
            Ok(text) => text,
            Err(e) => return format!("Failed to read response: {}", e),
        };

        // Try to parse as JSON for pretty formatting
        let formatted_body = if content_type.contains("json") {
            match serde_json::from_str::<Value>(&body_text) {
                Ok(json) => serde_json::to_string_pretty(&json).unwrap_or(body_text),
                Err(_) => body_text,
            }
        } else {
            // Truncate non-JSON responses
            if body_text.len() > 2000 {
                format!("{}...\n[Truncated, {} bytes total]", &body_text[..2000], body_text.len())
            } else {
                body_text
            }
        };

        // Format output
        let mut output = format!(
            "HTTP {} {}\nStatus: {} {}\n",
            http_method.as_str(),
            full_url,
            status_code,
            status.canonical_reason().unwrap_or("")
        );

        output.push_str("\nHeaders:\n");
        for (k, v) in response_headers {
            output.push_str(&format!("  {}: {}\n", k, v));
        }

        output.push_str("\nBody:\n");
        output.push_str(&formatted_body);

        output
    }

    // Tool dispatch
    async fn dispatch_tool(&self, name: &str, args: Option<Map<String, Value>>) -> CallToolResult {
        let args = args.unwrap_or_default();

        let result = match name {
            "auth_test" => self.auth_test().await,
            "auth_status" => self.auth_status().await,
            "auth_login" => self.auth_login().await,
            "auth_logout" => self.auth_logout().await,
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
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
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
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                self.admin_user_add(account_id, email, role, filter, dry_run)
                    .await
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
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                self.admin_user_remove(account_id, email, filter, dry_run)
                    .await
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
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                self.admin_user_update_role(account_id, email, role, filter, dry_run)
                    .await
            }
            "admin_operation_list" => {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.admin_operation_list(limit).await
            }
            "admin_operation_status" => {
                let operation_id = Self::optional_arg(&args, "operation_id");
                self.admin_operation_status(operation_id).await
            }
            "admin_folder_rights" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let email = match Self::required_arg(&args, "email") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let level = match Self::required_arg(&args, "level") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let folder = Self::optional_arg(&args, "folder");
                let filter = Self::optional_arg(&args, "filter");
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                self.admin_folder_rights(account_id, email, level, folder, filter, dry_run)
                    .await
            }
            "admin_operation_resume" => {
                let operation_id = Self::optional_arg(&args, "operation_id");
                self.admin_operation_resume(operation_id).await
            }
            "admin_operation_cancel" => {
                let operation_id = match Self::required_arg(&args, "operation_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.admin_operation_cancel(operation_id).await
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
                self.issue_create(project_id, title, description, status)
                    .await
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
                self.issue_update(project_id, issue_id, title, description, status)
                    .await
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
            "rfi_create" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = match Self::required_arg(&args, "title") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let question = Self::optional_arg(&args, "question");
                let priority = Self::optional_arg(&args, "priority");
                let due_date = Self::optional_arg(&args, "due_date");
                let assigned_to = Self::optional_arg(&args, "assigned_to");
                let location = Self::optional_arg(&args, "location");
                self.rfi_create(
                    project_id,
                    title,
                    question,
                    priority,
                    due_date,
                    assigned_to,
                    location,
                )
                .await
            }
            "rfi_update" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let rfi_id = match Self::required_arg(&args, "rfi_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = Self::optional_arg(&args, "title");
                let question = Self::optional_arg(&args, "question");
                let answer = Self::optional_arg(&args, "answer");
                let status = Self::optional_arg(&args, "status");
                let priority = Self::optional_arg(&args, "priority");
                self.rfi_update(
                    project_id, rfi_id, title, question, answer, status, priority,
                )
                .await
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
            "asset_create" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let category_id = Self::optional_arg(&args, "category_id");
                let description = Self::optional_arg(&args, "description");
                let barcode = Self::optional_arg(&args, "barcode");
                let client_asset_id = Self::optional_arg(&args, "client_asset_id");
                self.asset_create(
                    project_id,
                    category_id,
                    description,
                    barcode,
                    client_asset_id,
                )
                .await
            }
            "asset_update" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let asset_id = match Self::required_arg(&args, "asset_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let category_id = Self::optional_arg(&args, "category_id");
                let status_id = Self::optional_arg(&args, "status_id");
                let description = Self::optional_arg(&args, "description");
                let barcode = Self::optional_arg(&args, "barcode");
                self.asset_update(
                    project_id,
                    asset_id,
                    category_id,
                    status_id,
                    description,
                    barcode,
                )
                .await
            }
            "asset_delete" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let asset_id = match Self::required_arg(&args, "asset_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.asset_delete(project_id, asset_id).await
            }
            "acc_submittals_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.acc_submittals_list(project_id).await
            }
            "submittal_create" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = match Self::required_arg(&args, "title") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let spec_section = Self::optional_arg(&args, "spec_section");
                let due_date = Self::optional_arg(&args, "due_date");
                self.submittal_create(project_id, title, spec_section, due_date)
                    .await
            }
            "submittal_update" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let submittal_id = match Self::required_arg(&args, "submittal_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = Self::optional_arg(&args, "title");
                let status = Self::optional_arg(&args, "status");
                let due_date = Self::optional_arg(&args, "due_date");
                self.submittal_update(project_id, submittal_id, title, status, due_date)
                    .await
            }
            "acc_checklists_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.acc_checklists_list(project_id).await
            }
            "checklist_create" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = match Self::required_arg(&args, "title") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let template_id = Self::optional_arg(&args, "template_id");
                let location = Self::optional_arg(&args, "location");
                let due_date = Self::optional_arg(&args, "due_date");
                let assignee_id = Self::optional_arg(&args, "assignee_id");
                self.checklist_create(
                    project_id,
                    title,
                    template_id,
                    location,
                    due_date,
                    assignee_id,
                )
                .await
            }
            "checklist_update" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let checklist_id = match Self::required_arg(&args, "checklist_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let title = Self::optional_arg(&args, "title");
                let status = Self::optional_arg(&args, "status");
                let location = Self::optional_arg(&args, "location");
                let due_date = Self::optional_arg(&args, "due_date");
                let assignee_id = Self::optional_arg(&args, "assignee_id");
                self.checklist_update(
                    project_id,
                    checklist_id,
                    title,
                    status,
                    location,
                    due_date,
                    assignee_id,
                )
                .await
            }

            // ================================================================
            // Object Upload/Download Tools (v4.4)
            // ================================================================
            "object_upload" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let file_path = match Self::required_arg(&args, "file_path") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let object_key = Self::optional_arg(&args, "object_key");
                self.object_upload(bucket_key, file_path, object_key).await
            }
            "object_upload_batch" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let file_paths: Vec<String> = args
                    .get("file_paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                self.object_upload_batch(bucket_key, file_paths).await
            }
            "object_download" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let object_key = match Self::required_arg(&args, "object_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let output_path = match Self::required_arg(&args, "output_path") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.object_download(bucket_key, object_key, output_path)
                    .await
            }
            "object_info" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let object_key = match Self::required_arg(&args, "object_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.object_info(bucket_key, object_key).await
            }
            "object_copy" => {
                let source_bucket = match Self::required_arg(&args, "source_bucket") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let source_key = match Self::required_arg(&args, "source_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let dest_bucket = match Self::required_arg(&args, "dest_bucket") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let dest_key = Self::optional_arg(&args, "dest_key");
                self.object_copy(source_bucket, source_key, dest_bucket, dest_key)
                    .await
            }
            "object_delete_batch" => {
                let bucket_key = match Self::required_arg(&args, "bucket_key") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let object_keys: Vec<String> = args
                    .get("object_keys")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                self.object_delete_batch(bucket_key, object_keys).await
            }

            // ================================================================
            // Project Management Tools (v4.4)
            // ================================================================
            "project_info" => {
                let hub_id = match Self::required_arg(&args, "hub_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.project_info(hub_id, project_id).await
            }
            "project_users_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let offset = args
                    .get("offset")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.project_users_list(project_id, limit, offset).await
            }
            "folder_contents" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let folder_id = match Self::required_arg(&args, "folder_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let offset = args
                    .get("offset")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.folder_contents(project_id, folder_id, limit, offset)
                    .await
            }

            // ================================================================
            // ACC Project Admin Tools (v4.4)
            // ================================================================
            "project_create" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let name = match Self::required_arg(&args, "name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let template_project_id = Self::optional_arg(&args, "template_project_id");
                let products: Option<Vec<String>> =
                    args.get("products").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                self.project_create(account_id, name, template_project_id, products)
                    .await
            }
            "project_user_add" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let email = match Self::required_arg(&args, "email") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let role_id = Self::optional_arg(&args, "role_id");
                self.project_user_add(project_id, email, role_id).await
            }
            "project_users_import" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let users: Vec<Value> = args
                    .get("users")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                self.project_users_import(project_id, users).await
            }
            "project_update" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let name = Self::optional_arg(&args, "name");
                let status = Self::optional_arg(&args, "status");
                let start_date = Self::optional_arg(&args, "start_date");
                let end_date = Self::optional_arg(&args, "end_date");
                self.project_update(account_id, project_id, name, status, start_date, end_date)
                    .await
            }
            "project_archive" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.project_archive(account_id, project_id).await
            }
            "project_user_remove" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let user_id = match Self::required_arg(&args, "user_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.project_user_remove(project_id, user_id).await
            }
            "project_user_update" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let user_id = match Self::required_arg(&args, "user_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let role_id = Self::optional_arg(&args, "role_id");
                self.project_user_update(project_id, user_id, role_id).await
            }

            // ================================================================
            // Template Management Tools (v4.5)
            // ================================================================
            "template_list" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.template_list(account_id, limit).await
            }
            "template_info" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let template_id = match Self::required_arg(&args, "template_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.template_info(account_id, template_id).await
            }
            "template_create" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let name = match Self::required_arg(&args, "name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let products: Option<Vec<String>> =
                    args.get("products").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                self.template_create(account_id, name, products).await
            }
            "template_update" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let template_id = match Self::required_arg(&args, "template_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let name = Self::optional_arg(&args, "name");
                let status = Self::optional_arg(&args, "status");
                self.template_update(account_id, template_id, name, status).await
            }
            "template_archive" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let template_id = match Self::required_arg(&args, "template_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.template_archive(account_id, template_id).await
            }
            "template_convert" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.template_convert(account_id, project_id).await
            }

            // ================================================================
            // Item Management Tools (v4.4)
            // ================================================================
            "item_create" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let folder_id = match Self::required_arg(&args, "folder_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let display_name = match Self::required_arg(&args, "display_name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let storage_id = match Self::required_arg(&args, "storage_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.item_create(project_id, folder_id, display_name, storage_id)
                    .await
            }
            "item_delete" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let item_id = match Self::required_arg(&args, "item_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.item_delete(project_id, item_id).await
            }
            "item_rename" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let item_id = match Self::required_arg(&args, "item_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let new_name = match Self::required_arg(&args, "new_name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.item_rename(project_id, item_id, new_name).await
            }

            // ================================================================
            // Custom API Requests (v4.5)
            // ================================================================
            "api_request" => {
                let method = match Self::required_arg(&args, "method") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let endpoint = match Self::required_arg(&args, "endpoint") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let query: Option<Map<String, Value>> = args
                    .get("query")
                    .and_then(|v| v.as_object())
                    .cloned();
                let headers: Option<Map<String, Value>> = args
                    .get("headers")
                    .and_then(|v| v.as_object())
                    .cloned();
                let body: Option<Value> = args.get("body").cloned();
                self.api_request(method, endpoint, query, headers, body).await
            }

            _ => format!("Unknown tool: {}", name),
        };

        CallToolResult::success(vec![Content::text(result)])
    }
}

// Helper to format file size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
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
            "auth_login",
            "Get instructions for 3-legged OAuth login. Login requires browser interaction and must be done via CLI.",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "auth_logout",
            "Logout from 3-legged OAuth and clear stored tokens",
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
            "List projects in an ACC/BIM360 account with advanced filtering. Supports name patterns, status, platform, date ranges, and regions.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "filter": {"type": "string", "description": "Filter expression. Keys: name (glob with *), status (active|inactive|archived), platform (acc|bim360), created (>YYYY-MM-DD or <YYYY-MM-DD), region (us|emea). Example: 'name:*Hospital*,status:active,platform:acc,created:>2024-01-01'"},
                    "limit": {"type": "integer", "description": "Max projects (default: 100, max: 500)"}
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
        Tool::new(
            "admin_folder_rights",
            "Bulk update folder permissions for a user across multiple projects. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to update permissions for"},
                    "level": {"type": "string", "description": "Permission level: view_only, view_download, upload_only, view_download_upload, view_download_upload_edit, folder_control"},
                    "folder": {"type": "string", "description": "Folder type: project_files, plans, or custom folder ID"},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview changes without applying (default: false)"}
                }),
                &["account_id", "email", "level"],
            ),
        ),
        Tool::new(
            "admin_operation_resume",
            "Resume an interrupted bulk admin operation from where it left off.",
            schema(
                json!({
                    "operation_id": {"type": "string", "description": "Operation ID to resume (optional, defaults to most recent in-progress)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "admin_operation_cancel",
            "Cancel an in-progress bulk admin operation.",
            schema(
                json!({
                    "operation_id": {"type": "string", "description": "Operation ID to cancel"}
                }),
                &["operation_id"],
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
        Tool::new(
            "rfi_create",
            "Create a new RFI (Request for Information) in an ACC/BIM360 project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "RFI title"},
                    "question": {"type": "string", "description": "RFI question text"},
                    "priority": {"type": "string", "description": "Priority: low, normal, high, critical (default: normal)"},
                    "due_date": {"type": "string", "description": "Due date in YYYY-MM-DD format"},
                    "assigned_to": {"type": "string", "description": "User ID to assign the RFI to"},
                    "location": {"type": "string", "description": "Location reference"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "rfi_update",
            "Update an existing RFI.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "rfi_id": {"type": "string", "description": "The RFI ID to update"},
                    "title": {"type": "string", "description": "New title"},
                    "question": {"type": "string", "description": "Updated question"},
                    "answer": {"type": "string", "description": "Answer to the RFI"},
                    "status": {"type": "string", "description": "New status: draft, open, answered, closed, void"},
                    "priority": {"type": "string", "description": "New priority: low, normal, high, critical"}
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
            "asset_create",
            "Create a new asset in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "category_id": {"type": "string", "description": "Asset category ID"},
                    "description": {"type": "string", "description": "Asset description"},
                    "barcode": {"type": "string", "description": "Asset barcode"},
                    "client_asset_id": {"type": "string", "description": "Client-defined asset identifier"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "asset_update",
            "Update an existing asset.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "asset_id": {"type": "string", "description": "The asset ID to update"},
                    "category_id": {"type": "string", "description": "New category ID"},
                    "status_id": {"type": "string", "description": "New status ID"},
                    "description": {"type": "string", "description": "New description"},
                    "barcode": {"type": "string", "description": "New barcode"}
                }),
                &["project_id", "asset_id"],
            ),
        ),
        Tool::new(
            "asset_delete",
            "Delete an asset from a project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "asset_id": {"type": "string", "description": "The asset ID to delete"}
                }),
                &["project_id", "asset_id"],
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
            "submittal_create",
            "Create a new submittal in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "Submittal title"},
                    "spec_section": {"type": "string", "description": "Specification section reference"},
                    "due_date": {"type": "string", "description": "Due date in YYYY-MM-DD format"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "submittal_update",
            "Update an existing submittal.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "submittal_id": {"type": "string", "description": "The submittal ID to update"},
                    "title": {"type": "string", "description": "New title"},
                    "status": {"type": "string", "description": "New status"},
                    "due_date": {"type": "string", "description": "New due date"}
                }),
                &["project_id", "submittal_id"],
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
        Tool::new(
            "checklist_create",
            "Create a new checklist in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "Checklist title"},
                    "template_id": {"type": "string", "description": "Checklist template ID to use"},
                    "location": {"type": "string", "description": "Location reference"},
                    "due_date": {"type": "string", "description": "Due date in YYYY-MM-DD format"},
                    "assignee_id": {"type": "string", "description": "User ID to assign the checklist to"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "checklist_update",
            "Update an existing checklist.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "checklist_id": {"type": "string", "description": "The checklist ID to update"},
                    "title": {"type": "string", "description": "New title"},
                    "status": {"type": "string", "description": "New status"},
                    "location": {"type": "string", "description": "New location"},
                    "due_date": {"type": "string", "description": "New due date"},
                    "assignee_id": {"type": "string", "description": "New assignee user ID"}
                }),
                &["project_id", "checklist_id"],
            ),
        ),
        // ================================================================
        // Object Upload/Download Tools (v4.4)
        // ================================================================
        Tool::new(
            "object_upload",
            "Upload a file to an OSS bucket. Automatically uses chunked upload for files > 100MB.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Target bucket key (3-128 chars, lowercase)"},
                    "file_path": {"type": "string", "description": "Absolute path to the file to upload"},
                    "object_key": {"type": "string", "description": "Optional object key (defaults to filename)"}
                }),
                &["bucket_key", "file_path"],
            ),
        ),
        Tool::new(
            "object_upload_batch",
            "Upload multiple files to an OSS bucket. Uses 4 parallel uploads. Returns summary with individual results.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Target bucket key"},
                    "file_paths": {"type": "array", "items": {"type": "string"}, "description": "Array of absolute file paths to upload"}
                }),
                &["bucket_key", "file_paths"],
            ),
        ),
        Tool::new(
            "object_download",
            "Download an object from OSS to a local file path.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Source bucket key"},
                    "object_key": {"type": "string", "description": "Object key to download"},
                    "output_path": {"type": "string", "description": "Local file path to save the downloaded file"}
                }),
                &["bucket_key", "object_key", "output_path"],
            ),
        ),
        Tool::new(
            "object_info",
            "Get detailed metadata for an object including size, content type, SHA1 hash, and timestamps.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Bucket key"},
                    "object_key": {"type": "string", "description": "Object key"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "object_copy",
            "Copy an object from one bucket to another. If destination exists, returns existing object with warning (non-destructive).",
            schema(
                json!({
                    "source_bucket": {"type": "string", "description": "Source bucket key"},
                    "source_key": {"type": "string", "description": "Source object key"},
                    "dest_bucket": {"type": "string", "description": "Destination bucket key"},
                    "dest_key": {"type": "string", "description": "Destination object key (defaults to source key)"}
                }),
                &["source_bucket", "source_key", "dest_bucket"],
            ),
        ),
        Tool::new(
            "object_delete_batch",
            "Delete multiple objects from an OSS bucket. Returns summary with individual results.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Bucket key"},
                    "object_keys": {"type": "array", "items": {"type": "string"}, "description": "Array of object keys to delete"}
                }),
                &["bucket_key", "object_keys"],
            ),
        ),
        // ================================================================
        // Project Management Tools (v4.4)
        // ================================================================
        Tool::new(
            "project_info",
            "Get project details including name, type, scopes, and top-level folders. Requires 3-legged auth.",
            schema(
                json!({
                    "hub_id": {"type": "string", "description": "Hub ID (e.g., b.abc123)"},
                    "project_id": {"type": "string", "description": "Project ID (e.g., b.project123)"}
                }),
                &["hub_id", "project_id"],
            ),
        ),
        Tool::new(
            "project_users_list",
            "List users with access to a project with pagination. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "limit": {"type": "integer", "description": "Max results per page (default: 50, max: 200)"},
                    "offset": {"type": "integer", "description": "Starting index for pagination"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "folder_contents",
            "List all items and subfolders within a folder. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "folder_id": {"type": "string", "description": "Folder ID (URN format)"},
                    "limit": {"type": "integer", "description": "Max results per page (default: 50)"},
                    "offset": {"type": "integer", "description": "Starting index"}
                }),
                &["project_id", "folder_id"],
            ),
        ),
        // ================================================================
        // ACC Project Admin Tools (v4.4)
        // ================================================================
        Tool::new(
            "project_create",
            "Create a new ACC project from scratch or from a template. ACC only (not BIM 360). Polls until project is activated. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "name": {"type": "string", "description": "Project name"},
                    "template_project_id": {"type": "string", "description": "Optional template project ID to clone from"},
                    "products": {"type": "array", "items": {"type": "string"}, "description": "Products to enable (e.g., ['build', 'docs', 'model'])"}
                }),
                &["account_id", "name"],
            ),
        ),
        Tool::new(
            "project_user_add",
            "Add a user to an ACC project with optional role assignment. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "email": {"type": "string", "description": "User email address"},
                    "role_id": {"type": "string", "description": "Optional role ID to assign"}
                }),
                &["project_id", "email"],
            ),
        ),
        Tool::new(
            "project_users_import",
            "Import multiple users to an ACC project at once. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "users": {"type": "array", "items": {"type": "object", "properties": {"email": {"type": "string"}, "role_id": {"type": "string"}}, "required": ["email"]}, "description": "Array of users to import"}
                }),
                &["project_id", "users"],
            ),
        ),
        Tool::new(
            "project_update",
            "Update an ACC project's metadata (name, status, dates). Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "project_id": {"type": "string", "description": "Project ID to update"},
                    "name": {"type": "string", "description": "New project name"},
                    "status": {"type": "string", "description": "New status (active, archived, suspended)"},
                    "start_date": {"type": "string", "description": "Project start date (YYYY-MM-DD)"},
                    "end_date": {"type": "string", "description": "Project end date (YYYY-MM-DD)"}
                }),
                &["account_id", "project_id"],
            ),
        ),
        Tool::new(
            "project_archive",
            "Archive an ACC project (soft delete). Archived projects can be restored later. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "project_id": {"type": "string", "description": "Project ID to archive"}
                }),
                &["account_id", "project_id"],
            ),
        ),
        Tool::new(
            "project_user_remove",
            "Remove a user from an ACC project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "user_id": {"type": "string", "description": "User ID to remove from project"}
                }),
                &["project_id", "user_id"],
            ),
        ),
        Tool::new(
            "project_user_update",
            "Update a user's role in an ACC project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "user_id": {"type": "string", "description": "User ID to update"},
                    "role_id": {"type": "string", "description": "New role ID to assign"}
                }),
                &["project_id", "user_id"],
            ),
        ),
        // ================================================================
        // Template Management Tools (v4.5)
        // ================================================================
        Tool::new(
            "template_list",
            "List project templates in an ACC account. Templates are projects with classification='template' that can be used as blueprints. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "limit": {"type": "integer", "description": "Maximum results (default: 100, max: 200)"}
                }),
                &["account_id"],
            ),
        ),
        Tool::new(
            "template_info",
            "Get details of a project template including name, status, products, and member counts. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "template_id": {"type": "string", "description": "Template (project) ID"}
                }),
                &["account_id", "template_id"],
            ),
        ),
        Tool::new(
            "template_create",
            "Create a new project template. Templates can be used as blueprints when creating new projects via project_create. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "name": {"type": "string", "description": "Template name"},
                    "products": {"type": "array", "items": {"type": "string"}, "description": "Products to enable (e.g., ['build', 'docs', 'model'])"}
                }),
                &["account_id", "name"],
            ),
        ),
        Tool::new(
            "template_update",
            "Update a template's name or status. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "template_id": {"type": "string", "description": "Template (project) ID"},
                    "name": {"type": "string", "description": "New template name"},
                    "status": {"type": "string", "description": "New status (active, archived, suspended)"}
                }),
                &["account_id", "template_id"],
            ),
        ),
        Tool::new(
            "template_archive",
            "Archive a template (soft delete). Archived templates cannot be used for new projects. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "template_id": {"type": "string", "description": "Template (project) ID to archive"}
                }),
                &["account_id", "template_id"],
            ),
        ),
        Tool::new(
            "template_convert",
            "Convert a production project to a template. Note: ACC API may not support this operation - use template_create instead.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "project_id": {"type": "string", "description": "Production project ID to convert"}
                }),
                &["account_id", "project_id"],
            ),
        ),
        // ================================================================
        // Item Management Tools (v4.4)
        // ================================================================
        Tool::new(
            "item_create",
            "Create a new item in a project folder by linking an OSS storage object. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "folder_id": {"type": "string", "description": "Target folder ID (URN format)"},
                    "display_name": {"type": "string", "description": "Display name for the item"},
                    "storage_id": {"type": "string", "description": "OSS storage object URN"}
                }),
                &["project_id", "folder_id", "display_name", "storage_id"],
            ),
        ),
        Tool::new(
            "item_delete",
            "Delete an item from a project folder. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "item_id": {"type": "string", "description": "Item ID to delete"}
                }),
                &["project_id", "item_id"],
            ),
        ),
        Tool::new(
            "item_rename",
            "Update an item's display name. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "item_id": {"type": "string", "description": "Item ID"},
                    "new_name": {"type": "string", "description": "New display name"}
                }),
                &["project_id", "item_id", "new_name"],
            ),
        ),
        // ================================================================
        // Custom API Requests (v4.5)
        // ================================================================
        Tool::new(
            "api_request",
            "Execute custom HTTP request to APS API endpoints using current authentication. \
             Only APS domains are allowed (developer.api.autodesk.com, acc.autodesk.com, etc.). \
             Use for API endpoints not covered by other tools.",
            schema(
                json!({
                    "method": {"type": "string", "description": "HTTP method: GET, POST, PUT, PATCH, DELETE"},
                    "endpoint": {"type": "string", "description": "API endpoint path (e.g., /oss/v2/buckets) or full URL"},
                    "query": {"type": "object", "description": "Optional query parameters as key-value pairs"},
                    "headers": {"type": "object", "description": "Optional custom headers (cannot override Authorization)"},
                    "body": {"type": "object", "description": "Optional JSON request body (POST, PUT, PATCH only)"}
                }),
                &["method", "endpoint"],
            ),
        ),
    ]
}

// ServerHandler implementation
impl ServerHandler for RapsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "RAPS MCP Server v4.5 - Autodesk Platform Services CLI\n\n\
                Provides direct access to APS APIs (72 tools):\n\
                * auth_* - Authentication (2-legged and 3-legged OAuth)\n\
                * bucket_*, object_* - OSS storage operations (incl. upload/download/copy)\n\
                * translate_* - CAD model translation\n\
                * hub_*, project_* - Data Management & Project Info\n\
                * folder_*, item_* - Folder and file management\n\
                * project_create, project_user_* - ACC Project Admin\n\
                * template_* - Project template management (v4.5)\n\
                * admin_* - Bulk account administration\n\
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
