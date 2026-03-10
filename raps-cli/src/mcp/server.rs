// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Server implementation for RAPS
//!
//! Exposes APS API functionality as MCP tools for AI assistants.
//! Tool implementations are split across sibling modules:
//! - `tools_oss`   – Auth, OSS bucket/object, and translation tools
//! - `tools_dm`    – Hub, project, folder, item, and template tools
//! - `tools_admin` – Admin bulk ops, user listing, portfolio reports
//! - `tools_acc`   – Issues, RFIs, assets, submittals, checklists
//! - `tools_misc`  – Custom API, webhooks, design automation, reality capture
//! - `dispatch`    – Tool dispatch (routes tool name → handler)
//! - `definitions` – Tool schema definitions (`get_tools`)

use rmcp::{ServerHandler, ServiceExt, model::*, transport::stdio};
#[cfg(feature = "mcp-http")]
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use serde_json::{Map, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use raps_acc::{
    AccClient, IssuesClient, RfiClient, admin::AccountAdminClient,
    permissions::FolderPermissionsClient, users::ProjectUsersClient,
};
use raps_da::DesignAutomationClient;
use raps_derivative::DerivativeClient;
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;
use raps_oss::OssClient;
use raps_reality::RealityCaptureClient;
use raps_webhooks::WebhooksClient;

use super::definitions::get_tools;

/// Default concurrency for bulk MCP operations.
/// Tuned to ~20 to maximize throughput without tripping APS rate limits
/// (data-management: 100 req/min, account-admin: 100 req/min).
pub(crate) const MCP_BULK_CONCURRENCY: usize = 20;

/// Headers that should be stripped from API responses before returning to AI.
pub(crate) const SENSITIVE_HEADERS: &[&str] = &[
    "set-cookie",
    "www-authenticate",
    "authorization",
    "proxy-authorization",
    "cookie",
];

/// RAPS MCP Server
///
/// Provides AI assistants with direct access to Autodesk Platform Services.
#[derive(Clone)]
pub struct RapsServer {
    pub(crate) config: Arc<Config>,
    pub(crate) http_config: HttpClientConfig,
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
        let config = Config::from_env_lenient()?;
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

    /// Accessor for config (used by sibling tool modules).
    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    /// Accessor for HTTP client config (used by sibling tool modules).
    pub(crate) fn http_config(&self) -> &HttpClientConfig {
        &self.http_config
    }

    // ========================================================================
    // Client factories (double-checked locking for cached clients)
    // ========================================================================

    pub(crate) async fn get_auth_client(&self) -> AuthClient {
        if let Some(client) = self.auth_client.read().await.as_ref() {
            return client.clone();
        }

        let mut guard = self.auth_client.write().await;
        if guard.is_none() {
            *guard = Some(
                AuthClient::new_with_http_config(
                    (*self.config).clone(),
                    self.http_config.clone(),
                )
                .expect("HTTP client configuration was validated at startup"),
            );
        }
        guard
            .as_ref()
            .expect("client was just initialized above")
            .clone()
    }

    pub(crate) async fn get_oss_client(&self) -> OssClient {
        if let Some(client) = self.oss_client.read().await.as_ref() {
            return client.clone();
        }

        let auth = self.get_auth_client().await;
        let mut guard = self.oss_client.write().await;
        if guard.is_none() {
            *guard = Some(
                OssClient::new_with_http_config(
                    (*self.config).clone(),
                    auth,
                    self.http_config.clone(),
                )
                .expect("HTTP client configuration was validated at startup"),
            );
        }
        guard
            .as_ref()
            .expect("client was just initialized above")
            .clone()
    }

    pub(crate) async fn get_derivative_client(&self) -> DerivativeClient {
        if let Some(client) = self.derivative_client.read().await.as_ref() {
            return client.clone();
        }

        let auth = self.get_auth_client().await;
        let mut guard = self.derivative_client.write().await;
        if guard.is_none() {
            *guard = Some(
                DerivativeClient::new_with_http_config(
                    (*self.config).clone(),
                    auth,
                    self.http_config.clone(),
                )
                .expect("HTTP client configuration was validated at startup"),
            );
        }
        guard
            .as_ref()
            .expect("client was just initialized above")
            .clone()
    }

    pub(crate) async fn get_dm_client(&self) -> DataManagementClient {
        if let Some(client) = self.dm_client.read().await.as_ref() {
            return client.clone();
        }

        let auth = self.get_auth_client().await;
        let mut guard = self.dm_client.write().await;
        if guard.is_none() {
            *guard = Some(
                DataManagementClient::new_with_http_config(
                    (*self.config).clone(),
                    auth,
                    self.http_config.clone(),
                )
                .expect("HTTP client configuration was validated at startup"),
            );
        }
        guard
            .as_ref()
            .expect("client was just initialized above")
            .clone()
    }

    // On-demand clients (not cached, created fresh each time)

    pub(crate) async fn get_admin_client(&self) -> AccountAdminClient {
        let auth = self.get_auth_client().await;
        AccountAdminClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
        .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_users_client(&self) -> ProjectUsersClient {
        let auth = self.get_auth_client().await;
        ProjectUsersClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
        .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_issues_client(&self) -> IssuesClient {
        let auth = self.get_auth_client().await;
        IssuesClient::new_with_http_config((*self.config).clone(), auth, self.http_config.clone())
            .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_rfi_client(&self) -> RfiClient {
        let auth = self.get_auth_client().await;
        RfiClient::new_with_http_config((*self.config).clone(), auth, self.http_config.clone())
            .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_acc_client(&self) -> AccClient {
        let auth = self.get_auth_client().await;
        AccClient::new_with_http_config((*self.config).clone(), auth, self.http_config.clone())
            .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_permissions_client(&self) -> FolderPermissionsClient {
        let auth = self.get_auth_client().await;
        FolderPermissionsClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
        .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_webhooks_client(&self) -> WebhooksClient {
        let auth = self.get_auth_client().await;
        WebhooksClient::new_with_http_config((*self.config).clone(), auth, self.http_config.clone())
            .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_da_client(&self) -> DesignAutomationClient {
        let auth = self.get_auth_client().await;
        DesignAutomationClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
        .expect("HTTP client configuration was validated at startup")
    }

    pub(crate) async fn get_reality_client(&self) -> RealityCaptureClient {
        let auth = self.get_auth_client().await;
        raps_reality::RealityCaptureClient::new_with_http_config(
            (*self.config).clone(),
            auth,
            self.http_config.clone(),
        )
        .expect("HTTP client configuration was validated at startup")
    }

    // ========================================================================
    // Utility helpers
    // ========================================================================

    pub(crate) fn clamp_limit(limit: Option<usize>, default: usize, max: usize) -> usize {
        let limit = limit.unwrap_or(default).max(1);
        limit.min(max)
    }

    pub(crate) fn required_arg(args: &Map<String, Value>, key: &str) -> Result<String, String> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .ok_or_else(|| format!("Missing required argument '{}'.", key))
    }

    pub(crate) fn optional_arg(args: &Map<String, Value>, key: &str) -> Option<String> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
    }

    /// Validate that a URN looks like a base64-encoded APS URN.
    pub(crate) fn validate_urn(urn: &str) -> Result<(), String> {
        if urn.len() < 10 {
            return Err("URN is too short — expected a base64-encoded APS URN.".to_string());
        }
        if urn.contains(' ') {
            return Err("URN must not contain spaces.".to_string());
        }
        Ok(())
    }

    /// Validate that an ID looks like a GUID (with optional prefix like `b.`).
    #[allow(dead_code)]
    pub(crate) fn validate_id(value: &str, label: &str) -> Result<(), String> {
        // Allow prefixed IDs like "b.abc-123" or plain GUIDs
        let id_part = value.rsplit('.').next().unwrap_or(value);
        if id_part.len() < 8 {
            return Err(format!(
                "{} '{}' looks too short — expected a GUID or APS ID.",
                label, value
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Human-readable file-size formatting.
pub(crate) fn format_size(bytes: u64) -> String {
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

/// Validate that a file path is not pointing at a sensitive system location.
/// Returns Ok(()) if safe, Err(message) if the path should be rejected.
pub(crate) fn validate_file_path(path: &std::path::Path) -> Result<(), String> {
    let path_str = path.to_string_lossy().to_lowercase();

    // Block well-known sensitive paths
    let blocked_patterns = [
        ".ssh",
        ".gnupg",
        ".aws/credentials",
        ".env",
        "id_rsa",
        "id_ed25519",
        "authorized_keys",
        "known_hosts",
        "/etc/shadow",
        "/etc/passwd",
        "/etc/cron",
        "credentials.json",
        "secrets.json",
        "token.json",
    ];

    for pattern in &blocked_patterns {
        if path_str.contains(pattern) {
            return Err(format!(
                "Error: Path '{}' targets a sensitive location (matched '{}').\n\
                 MCP tools cannot read/write security-sensitive files.",
                path.display(),
                pattern
            ));
        }
    }

    Ok(())
}

// ============================================================================
// ServerHandler implementation
// ============================================================================

impl ServerHandler for RapsServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(format!(
                "RAPS MCP Server v{version} - Autodesk Platform Services CLI\n\n\
                    Provides direct access to APS APIs:\n\
                    * auth_* - Authentication (2-legged and 3-legged OAuth)\n\
                    * bucket_*, object_* - OSS storage operations (incl. upload/download/copy)\n\
                    * translate_* - CAD model translation\n\
                    * hub_*, project_* - Data Management & Project Info\n\
                    * folder_*, item_* - Folder and file management\n\
                    * project_create, project_user_* - ACC Project Admin\n\
                    * template_* - Project template management\n\
                    * admin_* - Bulk account administration\n\
                    * issue_*, rfi_* - ACC Issues and RFIs\n\
                    * acc_* - ACC Assets, Submittals, Checklists\n\
                    * da_* - Design Automation\n\
                    * reality_* - Reality Capture / Photogrammetry\n\
                    * webhook_* - Event subscriptions\n\
                    * api_request - Custom APS API calls\n\
                    * report_* - Portfolio reports\n\n\
                    Set APS_CLIENT_ID and APS_CLIENT_SECRET env vars.\n\
                    For 3-legged auth, run 'raps auth login' first.",
                version = env!("CARGO_PKG_VERSION"),
            ))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
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
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let result = self.dispatch_tool(&request.name, request.arguments).await;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::RapsServer;
    use serde_json::{Map, json};

    #[test]
    fn test_clamp_limit_defaults() {
        assert_eq!(RapsServer::clamp_limit(None, 100, 500), 100);
    }

    #[test]
    fn test_clamp_limit_user_value() {
        assert_eq!(RapsServer::clamp_limit(Some(50), 100, 500), 50);
    }

    #[test]
    fn test_clamp_limit_exceeds_max() {
        assert_eq!(RapsServer::clamp_limit(Some(999), 100, 500), 500);
    }

    #[test]
    fn test_clamp_limit_zero_becomes_one() {
        assert_eq!(RapsServer::clamp_limit(Some(0), 100, 500), 1);
    }

    #[test]
    fn test_required_arg_present() {
        let mut args = Map::new();
        args.insert("key".to_string(), json!("value"));
        let result = RapsServer::required_arg(&args, "key");
        assert_eq!(result, Ok("value".to_string()));
    }

    #[test]
    fn test_required_arg_missing() {
        let args = Map::new();
        let result = RapsServer::required_arg(&args, "key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing required"));
    }

    #[test]
    fn test_required_arg_empty_string() {
        let mut args = Map::new();
        args.insert("key".to_string(), json!(""));
        let result = RapsServer::required_arg(&args, "key");
        assert!(result.is_err());
    }

    #[test]
    fn test_required_arg_whitespace_only() {
        let mut args = Map::new();
        args.insert("key".to_string(), json!("   "));
        let result = RapsServer::required_arg(&args, "key");
        assert!(result.is_err());
    }

    #[test]
    fn test_required_arg_trims_whitespace() {
        let mut args = Map::new();
        args.insert("key".to_string(), json!("  value  "));
        let result = RapsServer::required_arg(&args, "key");
        assert_eq!(result, Ok("value".to_string()));
    }

    #[test]
    fn test_optional_arg_present() {
        let mut args = Map::new();
        args.insert("key".to_string(), json!("value"));
        assert_eq!(
            RapsServer::optional_arg(&args, "key"),
            Some("value".to_string())
        );
    }

    #[test]
    fn test_optional_arg_missing() {
        let args = Map::new();
        assert_eq!(RapsServer::optional_arg(&args, "key"), None);
    }

    #[test]
    fn test_optional_arg_empty_returns_none() {
        let mut args = Map::new();
        args.insert("key".to_string(), json!(""));
        assert_eq!(RapsServer::optional_arg(&args, "key"), None);
    }

    #[test]
    fn test_validate_urn_valid() {
        assert!(
            RapsServer::validate_urn("dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6YnVja2V0L2ZpbGUucnZ0")
                .is_ok()
        );
    }

    #[test]
    fn test_validate_urn_too_short() {
        assert!(RapsServer::validate_urn("abc").is_err());
    }

    #[test]
    fn test_validate_urn_with_spaces() {
        assert!(RapsServer::validate_urn("some urn value here").is_err());
    }
}

/// Run the MCP server using the specified transport
pub async fn run_server(transport: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // logging::init() in main.rs already set up a global subscriber;
    // no redundant init needed here.
    let _ = port; // used only with mcp-http feature

    match transport {
        "stdio" => {
            let server = RapsServer::new()?;
            let service = server.serve(stdio()).await?;
            service.waiting().await?;
        }
        #[cfg(feature = "mcp-http")]
        "http" => {
            let config = rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default();
            let service = StreamableHttpService::new(
                || RapsServer::new().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
                LocalSessionManager::default().into(),
                config,
            );
            let app = axum::Router::new().nest_service("/mcp", service);
            let addr = format!("0.0.0.0:{}", port);
            tracing::info!("MCP HTTP server listening on {}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
        #[cfg(not(feature = "mcp-http"))]
        "http" => {
            return Err("HTTP transport requires --features mcp-http".into());
        }
        other => {
            return Err(format!("Unknown transport: '{}'. Use 'stdio' or 'http'.", other).into());
        }
    }
    Ok(())
}
