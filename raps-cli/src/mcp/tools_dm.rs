// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Data Management tool implementations for the MCP server.

use serde_json::Value;

use super::server::RapsServer;

impl RapsServer {
    pub(crate) async fn hub_list(&self, limit: Option<usize>) -> String {
        let client = self.get_dm_client().await;
        let limit = Self::clamp_limit(limit, 50, 200);

        match client.list_hubs().await {
            Ok(all_hubs) => {
                let total = all_hubs.len();
                let hubs: Vec<_> = all_hubs.into_iter().take(limit).collect();
                let shown = hubs.len();
                let mut output = if shown < total {
                    format!("Showing {} of {} hub(s):\n\n", shown, total)
                } else {
                    format!("Found {} hub(s):\n\n", total)
                };
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

    pub(crate) async fn hub_info(&self, hub_id: String) -> String {
        let client = self.get_dm_client().await;

        match client.get_hub(&hub_id).await {
            Ok(hub) => {
                let region = hub.attributes.region.as_deref().unwrap_or("unknown");
                let hub_type = &hub.hub_type;
                format!(
                    "Hub details:\n\n* Name: {}\n* ID: {}\n* Type: {}\n* Region: {}",
                    hub.attributes.name, hub.id, hub_type, region
                )
            }
            Err(e) => format!("Failed to get hub info: {}", e),
        }
    }

    pub(crate) async fn project_list(&self, hub_id: String, limit: Option<usize>) -> String {
        let client = self.get_dm_client().await;
        let limit = Self::clamp_limit(limit, 50, 200);

        match client.list_projects(&hub_id).await {
            Ok(all_projects) => {
                let total = all_projects.len();
                let projects: Vec<_> = all_projects.into_iter().take(limit).collect();
                let shown = projects.len();
                let mut output = if shown < total {
                    format!("Showing {} of {} project(s):\n\n", shown, total)
                } else {
                    format!("Found {} project(s):\n\n", total)
                };
                for proj in &projects {
                    output.push_str(&format!("* {} (id: {})\n", proj.attributes.name, proj.id));
                }
                output
            }
            Err(e) => format!("Failed to list projects: {}", e),
        }
    }

    // ========================================================================
    // Folder/Item Management Tools
    // ========================================================================

    pub(crate) async fn folder_list(&self, project_id: String, folder_id: String) -> String {
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

    pub(crate) async fn folder_create(
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

    pub(crate) async fn item_info(&self, project_id: String, item_id: String) -> String {
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

    pub(crate) async fn item_versions(&self, project_id: String, item_id: String) -> String {
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
    // Project Management Operations (v4.4)
    // ========================================================================

    pub(crate) async fn project_info(&self, hub_id: String, project_id: String) -> String {
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

    pub(crate) async fn project_users_list(
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
                        .unwrap_or(user.role_ids.first().map(String::as_str).unwrap_or("-"));

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

    // Note: pagination is client-side (DM API fetches all pages then we skip/take).
    // Server-side pagination would require changes to the raps-dm crate.
    pub(crate) async fn folder_contents(
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
                        folders.push(format!("\u{1F4C1} {} ({})", name, id));
                    } else {
                        files.push(format!("\u{1F4C4} {}", name));
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

    pub(crate) async fn project_create(
        &self,
        account_id: String,
        name: String,
        template_project_id: Option<String>,
        products: Option<Vec<String>>,
    ) -> String {
        use raps_acc::CreateProjectRequest;

        let client = self.get_acc_client().await;

        // ACC auto-enables all products; only pass products if explicitly specified
        // and not empty (API rejects string product names, would need object format)
        let _ = products; // consumed but not used — ACC enables all products by default
        let request = CreateProjectRequest {
            name: name.clone(),
            template_project_id,
            products: None,
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

    pub(crate) async fn project_user_add(
        &self,
        project_id: String,
        email: String,
        role: Option<String>,
    ) -> String {
        use raps_acc::users::AddProjectUserRequest;

        let client = self.get_users_client().await;
        let role_name = role.as_deref().unwrap_or("member");

        // Resolve role name to products or UUID
        let admin_client = self.get_admin_client().await;
        let (role_ids, products) = match admin_client.resolve_role("", role_name).await {
            Ok(raps_acc::admin::ResolvedRole::Uuid(id)) => (vec![id], vec![]),
            Ok(raps_acc::admin::ResolvedRole::Products(prods)) => (vec![], prods),
            Err(_) => {
                // If resolution fails, try as raw UUID
                if role.is_some() {
                    (vec![role_name.to_string()], vec![])
                } else {
                    (vec![], vec![])
                }
            }
        };

        let request = AddProjectUserRequest {
            email: email.clone(),
            role_ids,
            products,
            suppress_administrative_emails: false,
        };

        match client.add_user(&project_id, request).await {
            Ok(user) => format!(
                "Added user to project:\n* Email: {}\n* Name: {}\n* Role: {}",
                email,
                user.name.unwrap_or_else(|| "-".to_string()),
                user.role_name
                    .unwrap_or_else(|| user.role_ids.into_iter().next().unwrap_or_else(|| "-".to_string()))
            ),
            Err(e) => format!("Failed to add user to project: {}", e),
        }
    }

    pub(crate) async fn project_users_import(
        &self,
        project_id: String,
        users: Vec<Value>,
    ) -> String {
        use raps_acc::users::ImportUserRequest;

        let client = self.get_users_client().await;

        // Parse user objects from JSON
        let import_requests: Vec<ImportUserRequest> = users
            .into_iter()
            .filter_map(|v| {
                let email = v.get("email")?.as_str()?.to_string();
                Some(ImportUserRequest {
                    email,
                    role_ids: v.get("role").and_then(|r| r.as_str()).map(|s| vec![s.to_string()]).unwrap_or_default(),
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
                    output.push_str(&format!("\u{2713} {}\n", success.email));
                }

                for error in &result.errors {
                    output.push_str(&format!("\u{2717} {} ({})\n", error.email, error.error));
                }

                output
            }
            Err(e) => format!("Failed to import users: {}", e),
        }
    }

    pub(crate) async fn project_update(
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

        match client
            .update_project(&account_id, &project_id, request)
            .await
        {
            Ok(project) => format!(
                "Updated project:\n* Name: {}\n* ID: {}\n* Status: {}",
                project.name,
                project.id,
                project.status.unwrap_or_else(|| "unknown".to_string())
            ),
            Err(e) => format!("Failed to update project: {}", e),
        }
    }

    pub(crate) async fn project_archive(&self, account_id: String, project_id: String) -> String {
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

    pub(crate) async fn project_user_remove(&self, project_id: String, user_id: String) -> String {
        let client = self.get_users_client().await;

        match client.remove_user(&project_id, &user_id).await {
            Ok(()) => format!(
                "Removed user from project:\n* User ID: {}\n* Project ID: {}",
                user_id, project_id
            ),
            Err(e) => format!("Failed to remove user from project: {}", e),
        }
    }

    pub(crate) async fn project_user_update(
        &self,
        project_id: String,
        user_id: String,
        role: Option<String>,
    ) -> String {
        use raps_acc::users::UpdateProjectUserRequest;

        let client = self.get_users_client().await;

        if role.is_none() {
            return "Error: role must be provided (e.g. admin, member, editor, viewer, or a UUID).".to_string();
        }
        let role_name = role.as_deref().unwrap();

        // Resolve role name to products or UUID
        let admin_client = self.get_admin_client().await;
        let (role_ids, products) = match admin_client.resolve_role("", role_name).await {
            Ok(raps_acc::admin::ResolvedRole::Uuid(id)) => (vec![id], None),
            Ok(raps_acc::admin::ResolvedRole::Products(prods)) => (vec![], Some(prods)),
            Err(_) => (vec![role_name.to_string()], None),
        };

        let request = UpdateProjectUserRequest {
            role_ids,
            products,
        };

        match client.update_user(&project_id, &user_id, request).await {
            Ok(user) => format!(
                "Updated user in project:\n* User ID: {}\n* Name: {}\n* Role: {}",
                user.id,
                user.name.unwrap_or_else(|| "-".to_string()),
                user.role_name
                    .unwrap_or_else(|| user.role_ids.into_iter().next().unwrap_or_else(|| "-".to_string()))
            ),
            Err(e) => format!("Failed to update user in project: {}", e),
        }
    }

    // ========================================================================
    // Template Management Operations (v4.5)
    // ========================================================================

    pub(crate) async fn template_list(&self, account_id: String, limit: Option<usize>) -> String {
        let client = self.get_admin_client().await;

        match client.list_templates(&account_id, limit, None).await {
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

    pub(crate) async fn template_info(&self, account_id: String, template_id: String) -> String {
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

    pub(crate) async fn template_create(
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

    pub(crate) async fn template_update(
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

        match client
            .update_project(&account_id, &template_id, request)
            .await
        {
            Ok(project) => format!(
                "Updated template:\n* Name: {}\n* ID: {}\n* Status: {}",
                project.name,
                project.id,
                project.status.unwrap_or_else(|| "unknown".to_string())
            ),
            Err(e) => format!("Failed to update template: {}", e),
        }
    }

    pub(crate) async fn template_archive(&self, account_id: String, template_id: String) -> String {
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

    pub(crate) async fn template_convert(&self, account_id: String, project_id: String) -> String {
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

    pub(crate) async fn item_create(
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

    pub(crate) async fn item_delete(&self, project_id: String, item_id: String) -> String {
        let client = self.get_dm_client().await;

        match client.delete_item(&project_id, &item_id).await {
            Ok(()) => format!("Deleted item from project:\n* Item ID: {}", item_id),
            Err(e) => format!("Failed to delete item: {}", e),
        }
    }

    pub(crate) async fn item_rename(
        &self,
        project_id: String,
        item_id: String,
        new_name: String,
    ) -> String {
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
}
