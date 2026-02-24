// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC (Issues, RFIs, Assets, Submittals, Checklists) tool implementations for the MCP server.

use raps_acc::{
    CreateAssetRequest, CreateChecklistRequest, CreateIssueRequest, CreateRfiRequest,
    CreateSubmittalRequest, UpdateAssetRequest, UpdateChecklistRequest, UpdateIssueRequest,
    UpdateRfiRequest, UpdateSubmittalRequest,
};

use super::server::RapsServer;

impl RapsServer {
    // ========================================================================
    // Issues Tools
    // ========================================================================

    pub(crate) async fn issue_list(&self, project_id: String, filter: Option<String>) -> String {
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

    pub(crate) async fn issue_get(&self, project_id: String, issue_id: String) -> String {
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

    pub(crate) async fn issue_create(
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

    pub(crate) async fn issue_update(
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
    // Issue Comment Tools
    // ========================================================================

    pub(crate) async fn issue_comments_list(&self, project_id: String, issue_id: String) -> String {
        let client = self.get_issues_client().await;

        match client.list_comments(&project_id, &issue_id).await {
            Ok(comments) => {
                if comments.is_empty() {
                    return "No comments found.".to_string();
                }

                let mut output = format!("Found {} comment(s):\n\n", comments.len());
                for comment in &comments {
                    let author = comment.created_by.as_deref().unwrap_or("-");
                    let created = comment.created_at.as_deref().unwrap_or("-");
                    output.push_str(&format!(
                        "* [{}] by {} ({})\n  {}\n",
                        comment.id, author, created, comment.body
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list issue comments: {}", e),
        }
    }

    pub(crate) async fn issue_comment_add(
        &self,
        project_id: String,
        issue_id: String,
        body: String,
    ) -> String {
        let client = self.get_issues_client().await;

        match client.add_comment(&project_id, &issue_id, &body).await {
            Ok(comment) => format!(
                "Comment added successfully:\n* ID: {}\n* Body: {}",
                comment.id, comment.body
            ),
            Err(e) => format!("Failed to add issue comment: {}", e),
        }
    }

    pub(crate) async fn issue_comment_delete(
        &self,
        project_id: String,
        issue_id: String,
        comment_id: String,
    ) -> String {
        let client = self.get_issues_client().await;

        match client
            .delete_comment(&project_id, &issue_id, &comment_id)
            .await
        {
            Ok(()) => format!("Comment {} deleted successfully.", comment_id),
            Err(e) => format!("Failed to delete issue comment: {}", e),
        }
    }

    // ========================================================================
    // RFI Tools
    // ========================================================================

    pub(crate) async fn rfi_list(&self, project_id: String) -> String {
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

    pub(crate) async fn rfi_get(&self, project_id: String, rfi_id: String) -> String {
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
    pub(crate) async fn rfi_create(
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
    pub(crate) async fn rfi_update(
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

    pub(crate) async fn acc_assets_list(&self, project_id: String) -> String {
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

    pub(crate) async fn asset_create(
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

    pub(crate) async fn asset_update(
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

    pub(crate) async fn asset_delete(&self, project_id: String, asset_id: String) -> String {
        let client = self.get_acc_client().await;

        match client.delete_asset(&project_id, &asset_id).await {
            Ok(()) => format!("Asset {} deleted successfully", asset_id),
            Err(e) => format!("Failed to delete asset: {}", e),
        }
    }

    pub(crate) async fn asset_get(&self, project_id: String, asset_id: String) -> String {
        let client = self.get_acc_client().await;

        match client.get_asset(&project_id, &asset_id).await {
            Ok(asset) => {
                let mut output = format!("Asset details:\n\n* ID: {}\n", asset.id);
                if let Some(ref desc) = asset.description {
                    output.push_str(&format!("* Description: {}\n", desc));
                }
                if let Some(ref status_id) = asset.status_id {
                    output.push_str(&format!("* Status ID: {}\n", status_id));
                }
                if let Some(ref category_id) = asset.category_id {
                    output.push_str(&format!("* Category ID: {}\n", category_id));
                }
                if let Some(ref barcode) = asset.barcode {
                    output.push_str(&format!("* Barcode: {}\n", barcode));
                }
                if let Some(ref created_at) = asset.created_at {
                    output.push_str(&format!("* Created: {}\n", created_at));
                }
                if let Some(ref updated_at) = asset.updated_at {
                    output.push_str(&format!("* Updated: {}\n", updated_at));
                }
                output
            }
            Err(e) => format!("Failed to get asset: {}", e),
        }
    }

    pub(crate) async fn acc_submittals_list(&self, project_id: String) -> String {
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

    pub(crate) async fn submittal_create(
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

    pub(crate) async fn submittal_update(
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

    pub(crate) async fn acc_checklists_list(&self, project_id: String) -> String {
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

    pub(crate) async fn checklist_templates_list(&self, project_id: String) -> String {
        let client = self.get_acc_client().await;

        match client.list_checklist_templates(&project_id).await {
            Ok(templates) => {
                if templates.is_empty() {
                    return "No checklist templates found.".to_string();
                }

                let mut output = format!("Found {} checklist template(s):\n\n", templates.len());
                for template in &templates {
                    let desc = template.description.as_deref().unwrap_or("-");
                    let created = template.created_at.as_deref().unwrap_or("-");
                    output.push_str(&format!(
                        "* {} (id: {}, created: {})\n  Description: {}\n",
                        template.title, template.id, created, desc
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list checklist templates: {}", e),
        }
    }

    pub(crate) async fn checklist_create(
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
    pub(crate) async fn checklist_update(
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
}
