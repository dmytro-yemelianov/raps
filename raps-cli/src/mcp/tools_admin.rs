// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Admin bulk operations tool implementations for the MCP server.

use std::sync::Arc;

use futures_util::stream::{self as stream_util, StreamExt as _};
use raps_admin::{BulkConfig, FolderType, PermissionLevel, ProjectFilter, StateManager};

use super::server::{MCP_BULK_CONCURRENCY, RapsServer};

impl RapsServer {
    pub(crate) async fn admin_project_list(
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

    pub(crate) async fn admin_user_add(
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
            concurrency: MCP_BULK_CONCURRENCY,
            dry_run,
            ..Default::default()
        };

        // Resolve role name to UUID (BIM 360) or product access list (ACC)
        let (resolved_role_id, resolved_products) = if let Some(ref role_name) = role {
            match admin_client.resolve_role(&account_id, role_name).await {
                Ok(raps_acc::admin::ResolvedRole::Uuid(id)) => (Some(id), vec![]),
                Ok(raps_acc::admin::ResolvedRole::Products(products)) => (None, products),
                Err(e) => return format!("Failed to resolve role: {}", e),
            }
        } else {
            (None, vec![])
        };

        // Progress callback (no-op for MCP)
        let on_progress = |_| {};

        match raps_admin::bulk_add_user(
            &admin_client,
            users_client,
            &account_id,
            &email,
            resolved_role_id.as_deref(),
            resolved_products,
            None,
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

    pub(crate) async fn admin_user_remove(
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
            concurrency: MCP_BULK_CONCURRENCY,
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

    pub(crate) async fn admin_user_update_role(
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
            concurrency: MCP_BULK_CONCURRENCY,
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

    pub(crate) async fn admin_folder_set_permissions(
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
            concurrency: MCP_BULK_CONCURRENCY,
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

    pub(crate) async fn admin_operation_list(&self, limit: Option<usize>) -> String {
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

    pub(crate) async fn admin_operation_status(&self, operation_id: Option<String>) -> String {
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

    pub(crate) async fn admin_operation_cancel(&self, operation_id: String) -> String {
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

    pub(crate) async fn admin_operation_resume(&self, operation_id: Option<String>) -> String {
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
            concurrency: MCP_BULK_CONCURRENCY,
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

    // ================================================================
    // Admin User Listing (v4.6)
    // ================================================================

    pub(crate) async fn admin_user_list(
        &self,
        account_id: String,
        project_id: Option<String>,
        search: Option<String>,
        role: Option<String>,
        status: Option<String>,
    ) -> String {
        if let Some(pid) = project_id {
            // Project-level user listing
            let client = self.get_users_client().await;
            match client.list_all_project_users(&pid).await {
                Ok(users) => {
                    let filtered: Vec<_> = users
                        .into_iter()
                        .filter(|u| {
                            if let Some(ref r) = role {
                                if let Some(ref rn) = u.role_name {
                                    if !rn.to_lowercase().contains(&r.to_lowercase()) {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            if let Some(ref s) = search {
                                let sl = s.to_lowercase();
                                let email_match = u
                                    .email
                                    .as_ref()
                                    .map(|e| e.to_lowercase().contains(&sl))
                                    .unwrap_or(false);
                                let name_match = u
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&sl))
                                    .unwrap_or(false);
                                if !email_match && !name_match {
                                    return false;
                                }
                            }
                            true
                        })
                        .collect();

                    let mut output =
                        format!("Found {} user(s) in project {}:\n\n", filtered.len(), pid);
                    for u in &filtered {
                        let email = u.email.as_deref().unwrap_or("N/A");
                        let name = u.name.as_deref().unwrap_or("N/A");
                        let role_name = u.role_name.as_deref().unwrap_or("N/A");
                        output.push_str(&format!("* {} ({}) - role: {}\n", email, name, role_name));
                    }
                    output
                }
                Err(e) => format!("Failed to list project users: {}", e),
            }
        } else {
            // Account-level user listing
            let client = self.get_admin_client().await;
            match client.list_all_users(&account_id).await {
                Ok(users) => {
                    let filtered: Vec<_> = users
                        .into_iter()
                        .filter(|u| {
                            if let Some(ref s) = status {
                                if let Some(ref us) = u.status {
                                    if !us.to_lowercase().eq(&s.to_lowercase()) {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            if let Some(ref s) = search {
                                let sl = s.to_lowercase();
                                let email_match = u.email.to_lowercase().contains(&sl);
                                let name_match = u
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&sl))
                                    .unwrap_or(false);
                                if !email_match && !name_match {
                                    return false;
                                }
                            }
                            true
                        })
                        .collect();

                    let mut output = format!(
                        "Found {} user(s) in account {}:\n\n",
                        filtered.len(),
                        account_id
                    );
                    for u in &filtered {
                        let name = u.display_name();
                        let status_str = u.status.as_deref().unwrap_or("unknown");
                        let company = u.company_id.as_deref().unwrap_or("N/A");
                        output.push_str(&format!(
                            "* {} ({}) - status: {}, company: {}\n",
                            u.email, name, status_str, company
                        ));
                    }
                    output
                }
                Err(e) => format!("Failed to list account users: {}", e),
            }
        }
    }

    // ================================================================
    // Portfolio Reports (v4.6)
    // ================================================================

    pub(crate) async fn report_rfi_summary(
        &self,
        account_id: String,
        filter: Option<String>,
        status_filter: Option<String>,
    ) -> String {
        let admin_client = self.get_admin_client().await;

        let project_filter = if let Some(ref f) = filter {
            match ProjectFilter::from_expression(f) {
                Ok(pf) => pf,
                Err(e) => return format!("Invalid filter expression: {}", e),
            }
        } else {
            ProjectFilter::new()
        };

        let projects = match admin_client.list_all_projects(&account_id).await {
            Ok(p) => project_filter.apply(p),
            Err(e) => return format!("Failed to list projects: {}", e),
        };

        if projects.is_empty() {
            return "No projects found matching filter.".to_string();
        }

        let rfi_client = self.get_rfi_client().await;
        let project_count = projects.len();

        // Fetch RFIs in parallel (bounded concurrency) instead of serial N+1
        let futs: Vec<_> = projects
            .into_iter()
            .map(|proj| {
                let client = rfi_client.clone();
                let sf = status_filter.clone();
                let proj_id = proj.id;
                let proj_name = proj.name;
                async move {
                    let res = client.list_rfis(&proj_id).await;
                    (proj_name, sf, res)
                }
            })
            .collect();

        let results: Vec<_> = stream_util::iter(futs)
            .buffer_unordered(MCP_BULK_CONCURRENCY)
            .collect()
            .await;

        let mut output = format!("RFI Summary for {} project(s):\n\n", project_count);
        let mut grand_total = 0usize;
        let mut grand_open = 0usize;

        for (proj_name, sf, res) in &results {
            match res {
                Ok(rfis) => {
                    let filtered: Vec<_> = if let Some(sf) = sf {
                        rfis.iter()
                            .filter(|r| r.status.to_lowercase() == sf.to_lowercase())
                            .collect()
                    } else {
                        rfis.iter().collect()
                    };
                    let total = filtered.len();
                    let open = filtered
                        .iter()
                        .filter(|r| r.status.to_lowercase() == "open")
                        .count();
                    grand_total += total;
                    grand_open += open;
                    if total > 0 {
                        output.push_str(&format!(
                            "* {} - {} RFIs ({} open)\n",
                            proj_name, total, open
                        ));
                    }
                }
                Err(_) => {
                    output.push_str(&format!("* {} - (access denied)\n", proj_name));
                }
            }
        }

        output.push_str(&format!(
            "\nTotal: {} RFIs across {} projects ({} open)",
            grand_total, project_count, grand_open
        ));
        output
    }

    pub(crate) async fn report_issues_summary(
        &self,
        account_id: String,
        filter: Option<String>,
        status_filter: Option<String>,
    ) -> String {
        let admin_client = self.get_admin_client().await;

        let project_filter = if let Some(ref f) = filter {
            match ProjectFilter::from_expression(f) {
                Ok(pf) => pf,
                Err(e) => return format!("Invalid filter expression: {}", e),
            }
        } else {
            ProjectFilter::new()
        };

        let projects = match admin_client.list_all_projects(&account_id).await {
            Ok(p) => project_filter.apply(p),
            Err(e) => return format!("Failed to list projects: {}", e),
        };

        if projects.is_empty() {
            return "No projects found matching filter.".to_string();
        }

        let issues_client = self.get_issues_client().await;
        let project_count = projects.len();

        // Fetch issues in parallel (bounded concurrency) instead of serial N+1
        let futs: Vec<_> = projects
            .into_iter()
            .map(|proj| {
                let client = issues_client.clone();
                let sf = status_filter.clone();
                let proj_id = proj.id;
                let proj_name = proj.name;
                async move {
                    let res = client.list_issues(&proj_id, None).await;
                    (proj_name, sf, res)
                }
            })
            .collect();

        let results: Vec<_> = stream_util::iter(futs)
            .buffer_unordered(MCP_BULK_CONCURRENCY)
            .collect()
            .await;

        let mut output = format!("Issues Summary for {} project(s):\n\n", project_count);
        let mut grand_total = 0usize;
        let mut grand_open = 0usize;

        for (proj_name, sf, res) in &results {
            match res {
                Ok(issues) => {
                    let filtered: Vec<_> = if let Some(sf) = sf {
                        issues
                            .iter()
                            .filter(|i| i.status.to_lowercase() == sf.to_lowercase())
                            .collect()
                    } else {
                        issues.iter().collect()
                    };
                    let total = filtered.len();
                    let open = filtered
                        .iter()
                        .filter(|i| i.status.to_lowercase() == "open")
                        .count();
                    grand_total += total;
                    grand_open += open;
                    if total > 0 {
                        output.push_str(&format!(
                            "* {} - {} issues ({} open)\n",
                            proj_name, total, open
                        ));
                    }
                }
                Err(_) => {
                    output.push_str(&format!("* {} - (access denied)\n", proj_name));
                }
            }
        }

        output.push_str(&format!(
            "\nTotal: {} issues across {} projects ({} open)",
            grand_total, project_count, grand_open
        ));
        output
    }
}
