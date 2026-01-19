// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication guidance module for MCP server
//!
//! Provides structured guidance content, tool-auth mappings, and helper functions
//! for native authentication support in the MCP server.

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;

/// Authentication requirement for MCP tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AuthRequirement {
    /// Requires 2-legged OAuth (client credentials)
    TwoLegged,
    /// Requires 3-legged OAuth (user authorization)
    ThreeLegged,
    /// Works with either auth type
    Either,
}

/// Current authentication state
#[derive(Debug, Clone)]
pub struct AuthState {
    /// Whether APS_CLIENT_ID is configured
    pub has_client_id: bool,
    /// Whether APS_CLIENT_SECRET is configured
    pub has_client_secret: bool,
    /// Whether 2-legged auth succeeds
    pub two_legged_valid: bool,
    /// Whether 3-legged token exists and is valid
    pub three_legged_valid: bool,
    /// Whether 3-legged token exists but is expired
    pub three_legged_expired: bool,
}

impl AuthState {
    /// Check if any credentials are configured
    #[allow(dead_code)]
    pub fn has_any_credentials(&self) -> bool {
        self.has_client_id || self.has_client_secret
    }

    /// Check if 2-legged credentials are complete
    #[allow(dead_code)]
    pub fn has_complete_2leg_credentials(&self) -> bool {
        self.has_client_id && self.has_client_secret
    }
}

// ============================================================================
// Instruction Constants
// ============================================================================

/// Full onboarding guide for first-time users
pub const SETUP_INSTRUCTIONS: &str = r#"
To set up authentication for RAPS MCP Server:

1. Go to https://aps.autodesk.com/ (Autodesk Platform Services)
2. Sign in or create an Autodesk account
3. Create a new application or select an existing one
4. Copy your Client ID and Client Secret
5. Set environment variables before starting the MCP server:

   For Unix/macOS:
     export APS_CLIENT_ID="your_client_id"
     export APS_CLIENT_SECRET="your_client_secret"

   For Windows (PowerShell):
     $env:APS_CLIENT_ID="your_client_id"
     $env:APS_CLIENT_SECRET="your_client_secret"

   Or add to your MCP server configuration (claude_desktop_config.json):
     {
       "mcpServers": {
         "raps": {
           "command": "raps",
           "args": ["serve"],
           "env": {
             "APS_CLIENT_ID": "your_client_id",
             "APS_CLIENT_SECRET": "your_client_secret"
           }
         }
       }
     }

For more information: https://rapscli.xyz/docs/auth
"#;

/// Missing client ID guidance
pub const MISSING_CLIENT_ID: &str = r#"
Missing: APS_CLIENT_ID environment variable

The Client ID is required for all APS operations. To get your Client ID:
1. Go to https://aps.autodesk.com/
2. Navigate to your application's settings
3. Copy the "Client ID" value
4. Set APS_CLIENT_ID environment variable
"#;

/// Missing client secret guidance
pub const MISSING_CLIENT_SECRET: &str = r#"
Missing: APS_CLIENT_SECRET environment variable

The Client Secret is required for authentication. To get your Client Secret:
1. Go to https://aps.autodesk.com/
2. Navigate to your application's settings
3. Copy or regenerate the "Client Secret" value
4. Set APS_CLIENT_SECRET environment variable

Note: Keep your Client Secret secure and never share it publicly.
"#;

/// Prompt to perform 3-legged authentication
pub const THREE_LEGGED_PROMPT: &str = r#"
To access BIM 360/ACC data (hubs, projects, folders, files, issues, RFIs), you need to log in with your Autodesk account.

Use the `auth_login` tool to start the authentication process, or run:
  raps auth login

This will open a browser window for you to sign in with your Autodesk credentials.
"#;

/// Tool availability summary header
pub const TOOL_AVAILABILITY_HEADER: &str = "\nTool Availability:\n";

// ============================================================================
// Tool-Auth Mapping
// ============================================================================

/// Get the authentication requirement for a tool
#[allow(dead_code)]
pub fn get_tool_auth_requirement(tool_name: &str) -> AuthRequirement {
    match tool_name {
        // Auth tools - work with either
        "auth_test" | "auth_status" | "auth_login" | "auth_logout" => AuthRequirement::Either,

        // OSS tools - 2-legged only (including v4.4 upload/download/copy)
        "bucket_list"
        | "bucket_create"
        | "bucket_get"
        | "bucket_delete"
        | "object_list"
        | "object_delete"
        | "object_signed_url"
        | "object_urn"
        | "object_upload"
        | "object_upload_batch"
        | "object_download"
        | "object_info"
        | "object_copy"
        | "object_delete_batch" => AuthRequirement::TwoLegged,

        // Derivative tools - 2-legged only
        "translate_start" | "translate_status" => AuthRequirement::TwoLegged,

        // Admin tools - 2-legged (account-level)
        "admin_project_list" | "admin_operation_list" | "admin_operation_status" => {
            AuthRequirement::TwoLegged
        }

        // Admin user tools - 2-legged
        "admin_user_add" | "admin_user_remove" | "admin_user_update_role" => {
            AuthRequirement::TwoLegged
        }

        // Data Management tools - 3-legged required
        "hub_list" | "project_list" | "folder_list" | "folder_create" | "item_info"
        | "item_versions" => AuthRequirement::ThreeLegged,

        // Project Management tools (v4.4) - 3-legged required
        "project_info" | "project_users_list" | "folder_contents" => AuthRequirement::ThreeLegged,

        // ACC Project Admin tools (v4.4) - 3-legged required
        "project_create" | "project_user_add" | "project_users_import" => {
            AuthRequirement::ThreeLegged
        }

        // Item Management tools (v4.4) - 3-legged required
        "item_create" | "item_delete" | "item_rename" => AuthRequirement::ThreeLegged,

        // ACC/BIM 360 tools - 3-legged required
        "issue_list"
        | "issue_get"
        | "issue_create"
        | "issue_update"
        | "rfi_list"
        | "rfi_get"
        | "acc_assets_list"
        | "acc_submittals_list"
        | "acc_checklists_list" => AuthRequirement::ThreeLegged,

        // Default to 2-legged for unknown tools
        _ => AuthRequirement::TwoLegged,
    }
}

// ============================================================================
// Auth State Helper
// ============================================================================

/// Compute current authentication state from config and auth client
pub async fn get_auth_state(config: &Config, auth_client: &AuthClient) -> AuthState {
    let has_client_id = !config.client_id.is_empty();
    let has_client_secret = !config.client_secret.is_empty();

    // Check 2-legged validity
    let two_legged_valid = if has_client_id && has_client_secret {
        auth_client.get_token().await.is_ok()
    } else {
        false
    };

    // Check 3-legged validity
    let (three_legged_valid, three_legged_expired) = match auth_client.get_3leg_token().await {
        Ok(_) => (true, false),
        Err(e) => {
            let err_msg = e.to_string().to_lowercase();
            if err_msg.contains("expired") || err_msg.contains("refresh") {
                (false, true)
            } else {
                (false, false)
            }
        }
    };

    AuthState {
        has_client_id,
        has_client_secret,
        two_legged_valid,
        three_legged_valid,
        three_legged_expired,
    }
}

// ============================================================================
// Error Guidance
// ============================================================================

/// Convert an authentication error into user-friendly guidance
pub fn format_error_guidance(error: &str) -> String {
    let error_lower = error.to_lowercase();

    if error_lower.contains("client_id") || error_lower.contains("aps_client_id") {
        return format!(
            "Authentication Error: Missing Client ID\n{}",
            MISSING_CLIENT_ID
        );
    }

    if error_lower.contains("client_secret") || error_lower.contains("aps_client_secret") {
        return format!(
            "Authentication Error: Missing Client Secret\n{}",
            MISSING_CLIENT_SECRET
        );
    }

    if error_lower.contains("401") || error_lower.contains("unauthorized") {
        return format!(
            r#"Authentication Error: Invalid Credentials

Your credentials were rejected by Autodesk. Troubleshooting steps:
1. Verify APS_CLIENT_ID is correct (typically 32 characters)
2. Verify APS_CLIENT_SECRET is correct (typically 64 characters)
3. Check that your app is active at https://aps.autodesk.com/
4. Ensure credentials are for the correct environment (production vs sandbox)

Error details: {}
"#,
            error
        );
    }

    if error_lower.contains("network")
        || error_lower.contains("connection")
        || error_lower.contains("timeout")
    {
        return format!(
            r#"Authentication Error: Network Issue

Unable to reach Autodesk authentication servers. Troubleshooting steps:
1. Check your internet connection
2. Verify firewall settings allow access to *.autodesk.com
3. Try again in a few moments

Error details: {}
"#,
            error
        );
    }

    if error_lower.contains("expired") || error_lower.contains("refresh") {
        return format!(
            r#"Authentication Error: Token Expired

Your 3-legged authentication token has expired. To re-authenticate:
1. Use the `auth_login` tool, or
2. Run `raps auth login` in your terminal

Error details: {}
"#,
            error
        );
    }

    // Generic fallback
    format!(
        r#"Authentication Error

An error occurred during authentication. Please check your setup:
1. Verify APS_CLIENT_ID and APS_CLIENT_SECRET are set correctly
2. Check your network connection
3. Visit https://rapscli.xyz/docs/troubleshooting for more help

Error details: {}
"#,
        error
    )
}

// ============================================================================
// Tool Availability Summary
// ============================================================================

/// Get a summary of tool availability based on auth state
pub fn get_tool_availability_summary(state: &AuthState) -> String {
    let mut summary = String::from(TOOL_AVAILABILITY_HEADER);

    // OSS tools
    if state.two_legged_valid {
        summary.push_str("  ✓ OSS (bucket_*, object_*) - available\n");
    } else {
        summary.push_str("  ✗ OSS (bucket_*, object_*) - requires 2-legged auth\n");
    }

    // Derivative tools
    if state.two_legged_valid {
        summary.push_str("  ✓ Derivative (translate_*) - available\n");
    } else {
        summary.push_str("  ✗ Derivative (translate_*) - requires 2-legged auth\n");
    }

    // Admin tools
    if state.two_legged_valid {
        summary.push_str("  ✓ Admin (admin_*) - available\n");
    } else {
        summary.push_str("  ✗ Admin (admin_*) - requires 2-legged auth\n");
    }

    // Data Management tools
    if state.three_legged_valid {
        summary.push_str("  ✓ Data Management (hub_*, project_*, folder_*, item_*) - available\n");
    } else {
        summary.push_str(
            "  ✗ Data Management (hub_*, project_*, folder_*, item_*) - requires 3-legged auth\n",
        );
    }

    // ACC tools
    if state.three_legged_valid {
        summary.push_str("  ✓ ACC (issue_*, rfi_*, acc_*) - available\n");
    } else {
        summary.push_str("  ✗ ACC (issue_*, rfi_*, acc_*) - requires 3-legged auth\n");
    }

    summary
}
