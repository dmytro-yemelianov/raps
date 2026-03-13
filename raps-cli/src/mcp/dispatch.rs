// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Tool dispatch implementation for the MCP server.

use raps_kernel::security::strip_prompt_injection;
use rmcp::model::*;
use serde_json::{Map, Value};

use super::server::RapsServer;

impl RapsServer {
    // Tool dispatch
    pub(crate) async fn dispatch_tool(
        &self,
        name: &str,
        args: Option<Map<String, Value>>,
    ) -> CallToolResult {
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
                let minutes = args
                    .get("minutes")
                    .and_then(|v| {
                        v.as_u64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(10) as u32;
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
                if let Err(err) = Self::validate_urn(&urn) {
                    return CallToolResult::success(vec![Content::text(err)]);
                }
                let format =
                    Self::optional_arg(&args, "format").unwrap_or_else(|| "svf2".to_string());
                self.translate_start(urn, format).await
            }
            "translate_status" => {
                let urn = match Self::required_arg(&args, "urn") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                if let Err(err) = Self::validate_urn(&urn) {
                    return CallToolResult::success(vec![Content::text(err)]);
                }
                self.translate_status(urn).await
            }
            "hub_list" => {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                self.hub_list(limit).await
            }
            "hub_info" => {
                let hub_id = match Self::required_arg(&args, "hub_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.hub_info(hub_id).await
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
            "admin_folder_set_permissions" => {
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
                self.admin_folder_set_permissions(account_id, email, level, folder, filter, dry_run)
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
            // Issue Comment Tools
            // ================================================================
            "issue_comments_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let issue_id = match Self::required_arg(&args, "issue_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.issue_comments_list(project_id, issue_id).await
            }
            "issue_comment_add" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let issue_id = match Self::required_arg(&args, "issue_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let body = match Self::required_arg(&args, "body") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.issue_comment_add(project_id, issue_id, body).await
            }
            "issue_comment_delete" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let issue_id = match Self::required_arg(&args, "issue_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let comment_id = match Self::required_arg(&args, "comment_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.issue_comment_delete(project_id, issue_id, comment_id)
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
            "asset_get" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let asset_id = match Self::required_arg(&args, "asset_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.asset_get(project_id, asset_id).await
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
            "checklist_templates_list" => {
                let project_id = match Self::required_arg(&args, "project_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.checklist_templates_list(project_id).await
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
                let role = Self::optional_arg(&args, "role");
                self.project_user_add(project_id, email, role).await
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
                let role = Self::optional_arg(&args, "role");
                self.project_user_update(project_id, user_id, role).await
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
                self.template_update(account_id, template_id, name, status)
                    .await
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
                let query: Option<Map<String, Value>> =
                    args.get("query").and_then(|v| v.as_object()).cloned();
                let headers: Option<Map<String, Value>> =
                    args.get("headers").and_then(|v| v.as_object()).cloned();
                let body: Option<Value> = args.get("body").cloned();
                self.api_request(method, endpoint, query, headers, body)
                    .await
            }

            // ================================================================
            // Webhooks (v4.6)
            // ================================================================
            "webhook_list" => self.webhook_list().await,
            "webhook_create" => {
                let system = match Self::required_arg(&args, "system") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let event = match Self::required_arg(&args, "event") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let callback_url = match Self::required_arg(&args, "callback_url") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let folder_urn = Self::optional_arg(&args, "folder_urn");
                self.webhook_create(system, event, callback_url, folder_urn)
                    .await
            }
            "webhook_delete" => {
                let system = match Self::required_arg(&args, "system") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let event = match Self::required_arg(&args, "event") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let hook_id = match Self::required_arg(&args, "hook_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.webhook_delete(system, event, hook_id).await
            }
            "webhook_events" => self.webhook_events().await,
            "webhook_get" => {
                let system = match Self::required_arg(&args, "system") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let event = match Self::required_arg(&args, "event") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let hook_id = match Self::required_arg(&args, "hook_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.webhook_get(system, event, hook_id).await
            }
            "webhook_update" => {
                let system = match Self::required_arg(&args, "system") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let event = match Self::required_arg(&args, "event") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let hook_id = match Self::required_arg(&args, "hook_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let callback_url = Self::optional_arg(&args, "callback_url");
                let status = Self::optional_arg(&args, "status");
                let filter = Self::optional_arg(&args, "filter");
                self.webhook_update(system, event, hook_id, callback_url, status, filter)
                    .await
            }

            // ================================================================
            // Design Automation (v4.6)
            // ================================================================
            "da_engines_list" => self.da_engines_list().await,
            "da_appbundles_list" => self.da_appbundles_list().await,
            "da_activities_list" => self.da_activities_list().await,
            "da_workitem_create" => {
                let activity_id = match Self::required_arg(&args, "activity_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let arguments: std::collections::HashMap<String, raps_da::WorkItemArgument> = args
                    .get("arguments")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| {
                                // Accept either {"url": "..."} object or plain "url" string
                                let arg = if let Some(url) = v.as_str() {
                                    raps_da::WorkItemArgument {
                                        url: url.to_string(),
                                        verb: None,
                                        headers: None,
                                    }
                                } else if let Some(obj) = v.as_object() {
                                    let url = obj
                                        .get("url")
                                        .and_then(|u| u.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let verb = obj
                                        .get("verb")
                                        .and_then(|u| u.as_str())
                                        .map(|s| s.to_string());
                                    raps_da::WorkItemArgument {
                                        url,
                                        verb,
                                        headers: None,
                                    }
                                } else {
                                    return None;
                                };
                                Some((k.clone(), arg))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.da_workitem_create(activity_id, arguments).await
            }
            "da_workitem_status" => {
                let workitem_id = match Self::required_arg(&args, "workitem_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.da_workitem_status(workitem_id).await
            }
            "da_workitems_list" => self.da_workitems_list().await,

            // ================================================================
            // Reality Capture
            // ================================================================
            "reality_list" => self.reality_list().await,
            "reality_create" => {
                let name = match Self::required_arg(&args, "name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let scene_type = Self::optional_arg(&args, "scene_type");
                let format = Self::optional_arg(&args, "format");
                self.reality_create(name, scene_type, format).await
            }
            "reality_process" => {
                let photoscene_id = match Self::required_arg(&args, "photoscene_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.reality_process(photoscene_id).await
            }
            "reality_status" => {
                let photoscene_id = match Self::required_arg(&args, "photoscene_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.reality_status(photoscene_id).await
            }
            "reality_result" => {
                let photoscene_id = match Self::required_arg(&args, "photoscene_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let format = Self::optional_arg(&args, "format");
                self.reality_result(photoscene_id, format).await
            }
            "reality_delete" => {
                let photoscene_id = match Self::required_arg(&args, "photoscene_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.reality_delete(photoscene_id).await
            }
            "reality_formats" => self.reality_formats().await,

            // ================================================================
            // Admin User Listing (v4.6)
            // ================================================================
            "admin_user_list" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let project_id = Self::optional_arg(&args, "project_id");
                let search = Self::optional_arg(&args, "search");
                let role = Self::optional_arg(&args, "role");
                let status = Self::optional_arg(&args, "status");
                self.admin_user_list(account_id, project_id, search, role, status)
                    .await
            }

            // ================================================================
            // Portfolio Reports (v4.6)
            // ================================================================
            "report_rfi_summary" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let filter = Self::optional_arg(&args, "filter");
                let status = Self::optional_arg(&args, "status");
                self.report_rfi_summary(account_id, filter, status).await
            }
            "report_issues_summary" => {
                let account_id = match Self::required_arg(&args, "account_id") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let filter = Self::optional_arg(&args, "filter");
                let status = Self::optional_arg(&args, "status");
                self.report_issues_summary(account_id, filter, status).await
            }

            // ================================================================
            // Pipeline v2 Tools
            // ================================================================
            "pipeline_validate" => {
                let file = match Self::required_arg(&args, "file") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.pipeline_validate(file).await
            }
            "pipeline_dry_run" => {
                let file = match Self::required_arg(&args, "file") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let variables = Self::optional_arg(&args, "variables");
                self.pipeline_dry_run(file, variables).await
            }
            "pipeline_run" => {
                let file = match Self::required_arg(&args, "file") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let ignore_failure = args
                    .get("ignore_failure")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let variables = Self::optional_arg(&args, "variables");
                self.pipeline_run(file, dry_run, ignore_failure, variables)
                    .await
            }
            "pipeline_list_templates" => {
                let directory = Self::optional_arg(&args, "directory");
                self.pipeline_list_templates(directory).await
            }

            // ─── Compound Workflows ─────────────────────────────────
            "workflow_prepare_for_viewing" => {
                let file_path = match Self::required_arg(&args, "file_path") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let bucket_key = Self::optional_arg(&args, "bucket_key");
                let object_key = Self::optional_arg(&args, "object_key");
                self.workflow_prepare_for_viewing(file_path, bucket_key, object_key)
                    .await
            }
            "workflow_analyze_model" => {
                let urn = match Self::required_arg(&args, "urn") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let region = Self::optional_arg(&args, "region");
                self.workflow_analyze_model(urn, region).await
            }
            "workflow_batch_translate" => {
                let urns = match Self::required_arg(&args, "urns") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let output_format = Self::optional_arg(&args, "output_format");
                let region = Self::optional_arg(&args, "region");
                self.workflow_batch_translate(urns, output_format, region)
                    .await
            }
            "workflow_compare_versions" => {
                let urn_a = match Self::required_arg(&args, "urn_a") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let urn_b = match Self::required_arg(&args, "urn_b") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let label_a = Self::optional_arg(&args, "label_a");
                let label_b = Self::optional_arg(&args, "label_b");
                self.workflow_compare_versions(urn_a, urn_b, label_a, label_b)
                    .await
            }
            "workflow_setup_project" => {
                let project_name = match Self::required_arg(&args, "project_name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let bucket_region = Self::optional_arg(&args, "bucket_region");
                self.workflow_setup_project(project_name, bucket_region)
                    .await
            }
            "swarm_status" => self.swarm_status_tool().await,

            // ── Skill tools ──────────────────────────────────────────
            "skill_list" => {
                let filter = Self::optional_arg(&args, "filter");
                self.skill_list(filter).await
            }
            "skill_install" => {
                let name = match Self::required_arg(&args, "name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
                self.skill_install(name, force).await
            }
            "skill_info" => {
                let name = match Self::required_arg(&args, "name") {
                    Ok(val) => val,
                    Err(err) => return CallToolResult::success(vec![Content::text(err)]),
                };
                self.skill_info(name).await
            }

            _ => format!("Unknown tool: {}", name),
        };

        // Enrich auth-related errors with actionable guidance
        let result = Self::enrich_error(&result);

        // Sanitize response body: parse as JSON → walk tree redacting injection patterns → re-serialize.
        // Falls back to the raw string if it isn't valid JSON (e.g., plain-text error messages).
        let sanitized_text = if let Ok(value) = serde_json::from_str::<serde_json::Value>(&result) {
            let sanitized = strip_prompt_injection(value);
            serde_json::to_string_pretty(&sanitized).unwrap_or(result)
        } else {
            result
        };

        CallToolResult::success(vec![Content::text(sanitized_text)])
    }

    /// If the result string looks like an auth error, append user-friendly guidance.
    /// Only triggers on actual error responses (prefixed with error markers), not successful data.
    fn enrich_error(result: &str) -> String {
        const ERROR_PREFIXES: &[&str] = &[
            "Failed to",
            "Error:",
            "Error ",
            "Authentication Error",
            "Auth error",
        ];

        let looks_like_error = ERROR_PREFIXES
            .iter()
            .any(|prefix| result.starts_with(prefix));

        if !looks_like_error {
            return result.to_string();
        }

        const AUTH_KEYWORDS: &[&str] = &[
            "401",
            "unauthorized",
            "client_id",
            "client_secret",
            "expired",
            "authentication failed",
            "not configured or invalid",
        ];

        let lower = result.to_lowercase();
        let is_auth_error = AUTH_KEYWORDS.iter().any(|kw| lower.contains(kw));

        if is_auth_error {
            let guidance = super::auth_guidance::format_error_guidance(result);
            format!("{result}\n\n{guidance}")
        } else {
            result.to_string()
        }
    }
}
