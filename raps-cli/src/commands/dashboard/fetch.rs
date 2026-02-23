// SPDX-License-Identifier: Apache-2.0
// Background data fetching for the TUI dashboard

use super::*;

/// Load data for the current view: serves from cache if fresh, else fetches in background.
/// When `force` is true, always fetches (used by manual refresh `r`).
pub(super) fn load_view(
    app: &mut App,
    clients: &Arc<Clients>,
    tx: &mpsc::UnboundedSender<BackgroundMsg>,
    force: bool,
) {
    if !app.has_current_view() {
        return;
    }
    let view = app.current_view().clone();

    // Check cache first (unless force-refreshing)
    if !force {
        if let Some(entry) = app.cache.get(&view) {
            if entry.is_fresh() {
                // Cache hit — restore data + cursor instantly
                app.data = Some(entry.data.clone());
                app.last_refresh = Some(entry.fetched_at);
                app.table_state
                    .select(entry.table_selection.or(Some(0)));
                let count = app.row_count();
                app.set_status(format!("{count} items (cached)"));
                app.loading = false;
                return;
            }
            // Stale cache — show stale data immediately while refetching
            app.data = Some(entry.data.clone());
            app.table_state
                .select(entry.table_selection.or(Some(0)));
            app.set_status("Refreshing stale data...");
        } else {
            // No cache at all
            app.data = None;
            app.table_state.select(Some(0));
        }
    } else {
        // Force refresh: keep showing current data while fetching
        app.set_status("Refreshing...");
    }

    // Throttle API calls — skip if we just fetched (unless force).
    // Exception: never throttle when there's no data to display (new uncached view),
    // because the user would see "Loading..." forever until the throttle expires.
    if !force
        && app.data.is_some()
        && app.last_fetch.is_some_and(|t| t.elapsed() < API_THROTTLE)
    {
        return;
    }

    app.loading = true;
    app.last_fetch = Some(Instant::now());
    spawn_fetch(&view, clients, tx, app.hub_context.clone());
}

/// Spawn a background task to fetch data for the given view.
fn spawn_fetch(
    view: &ViewKind,
    clients: &Arc<Clients>,
    tx: &mpsc::UnboundedSender<BackgroundMsg>,
    hub_context: Option<String>,
) {
    let view = view.clone();
    let clients = Arc::clone(clients);
    let tx = tx.clone();

    tokio::spawn(async move {
        let view_title = view.title();
        let _ = tx.send(BackgroundMsg::Log(format!("Fetching {view_title}...")));
        let result = fetch_data(&view, &clients, hub_context.as_deref()).await;
        match result {
            Ok(data) => {
                let _ = tx.send(BackgroundMsg::DataReady(view, data));
            }
            Err(e) => {
                let err_msg = format!("{e:#}");
                let _ = tx.send(BackgroundMsg::Log(format!("FAIL {view_title}: {err_msg}")));
                let _ = tx.send(BackgroundMsg::Error(err_msg));
            }
        }
    });
}

pub(super) fn handle_bg_msg(app: &mut App, msg: BackgroundMsg) {
    match msg {
        BackgroundMsg::DataReady(view, data) => {
            app.api_calls += 1;
            let now = Instant::now();
            // Always update cache regardless of current view
            app.cache.insert(
                view.clone(),
                CacheEntry {
                    data: data.clone(),
                    fetched_at: now,
                    table_selection: None,
                },
            );
            // Only update display if it matches the current view
            if app.has_current_view() && &view == app.current_view() {
                app.data = Some(data);
                app.loading = false;
                app.last_refresh = Some(now);
                let count = app.row_count();
                app.set_status(format!("{count} items loaded"));
                if app.table_state.selected().is_none() && count > 0 {
                    app.table_state.select(Some(0));
                }
            }
        }
        BackgroundMsg::Error(e) => {
            app.loading = false;
            app.push_log(format!("ERR: {e}"));
            app.error_msg = Some(e);
        }
        BackgroundMsg::Log(msg) => {
            app.push_log(msg);
        }
    }
}

/// Check if a string looks like a valid GUID/UUID (8-4-4-4-12 hex).
fn is_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12
        && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Resolve the ACC/BIM360 project ID for Issues/RFI API calls.
async fn resolve_acc_project_id(
    clients: &Clients,
    project_id: &str,
    hub_context: Option<&str>,
) -> Result<String> {
    // First try: strip "b." prefix (standard ACC pattern)
    let stripped = project_id
        .strip_prefix("b.")
        .unwrap_or(project_id);

    if is_guid(stripped) {
        tracing::info!("ACC project ID (stripped): {stripped}");
        return Ok(stripped.to_string());
    }

    // The stripped ID isn't a GUID — try to get the issues container ID
    // from the project relationships (BIM360 pattern).
    if let Some(hub_id) = hub_context {
        match clients.dm.get_issues_container_id(hub_id, project_id).await {
            Ok(Some(container_id)) if is_guid(&container_id) => {
                tracing::info!(
                    "Resolved issues container GUID: project={project_id} → container={container_id}"
                );
                return Ok(container_id);
            }
            Ok(Some(non_guid)) => {
                tracing::warn!(
                    "Issues container is not a GUID: {non_guid}"
                );
            }
            Ok(None) => {
                tracing::info!("No issues container in project relationships");
            }
            Err(e) => {
                tracing::warn!("Failed to get issues container ID: {e:#}");
            }
        }
    }

    anyhow::bail!(
        "Project ID '{project_id}' is not a valid GUID after stripping prefix. \
         This project may not support the ACC Issues API. \
         Try using the project's ACC UUID directly with :p <uuid>"
    );
}

/// Views that require 3-legged (user) auth.
fn requires_3leg(view: &ViewKind) -> bool {
    matches!(
        view,
        ViewKind::HubList
            | ViewKind::ProjectList { .. }
            | ViewKind::FolderList { .. }
            | ViewKind::ItemDetail { .. }
            | ViewKind::IssueList { .. }
            | ViewKind::IssueDetail { .. }
            | ViewKind::IssueCommentList { .. }
            | ViewKind::IssueAttachmentList { .. }
            | ViewKind::IssueTypeList { .. }
            | ViewKind::RfiList { .. }
            | ViewKind::RfiDetail { .. }
            | ViewKind::AssetList { .. }
            | ViewKind::AssetDetail { .. }
            | ViewKind::SubmittalList { .. }
            | ViewKind::SubmittalDetail { .. }
            | ViewKind::ChecklistList { .. }
            | ViewKind::ChecklistDetail { .. }
    )
}

async fn fetch_data(
    view: &ViewKind,
    clients: &Clients,
    hub_context: Option<&str>,
) -> Result<ResourceData> {
    if requires_3leg(view) && !clients.auth.is_logged_in().await {
        anyhow::bail!(
            "This view requires 3-legged authentication.\n\
             Run :login to sign in with your Autodesk account."
        );
    }

    match view {
        ViewKind::BucketList => {
            let buckets = clients.oss.list_buckets().await?;
            let rows: Vec<BucketRow> = buckets
                .into_iter()
                .map(|b| BucketRow {
                    key: b.bucket_key,
                    policy: b.policy_key,
                    created: util::format_timestamp(b.created_date),
                })
                .collect();
            Ok(ResourceData::Buckets(rows))
        }
        ViewKind::BucketDetail { bucket_key } => {
            let detail = clients.oss.get_bucket_details(bucket_key).await?;
            let fields = vec![
                DetailField { label: "Bucket Key".into(), value: detail.bucket_key },
                DetailField { label: "Owner".into(), value: detail.bucket_owner },
                DetailField { label: "Policy".into(), value: detail.policy_key },
                DetailField { label: "Created".into(), value: util::format_timestamp(detail.created_date) },
                DetailField { label: "Permissions".into(), value: format!("{:?}", detail.permissions) },
            ];
            Ok(ResourceData::BucketDetail(fields))
        }
        ViewKind::ObjectList { bucket_key } => {
            let objects = clients.oss.list_objects(bucket_key).await?;
            let rows: Vec<ObjectRow> = objects
                .into_iter()
                .map(|o| ObjectRow {
                    key: o.object_key,
                    size: util::format_size(o.size),
                    sha1: o.sha1.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Objects(rows))
        }
        ViewKind::ObjectDetail {
            bucket_key,
            object_key,
        } => {
            let detail = clients.oss.get_object_details(bucket_key, object_key).await?;
            let fields = vec![
                DetailField { label: "Bucket".into(), value: detail.bucket_key },
                DetailField { label: "Object Key".into(), value: detail.object_key },
                DetailField { label: "Object ID".into(), value: detail.object_id },
                DetailField { label: "Size".into(), value: util::format_size(detail.size) },
                DetailField { label: "SHA1".into(), value: detail.sha1 },
                DetailField { label: "Content Type".into(), value: detail.content_type },
                DetailField { label: "Location".into(), value: detail.location.unwrap_or_default() },
            ];
            Ok(ResourceData::ObjectDetail(fields))
        }
        ViewKind::HubList => {
            let hubs = clients.dm.list_hubs().await?;
            let rows: Vec<HubRow> = hubs
                .into_iter()
                .map(|h| HubRow {
                    name: h.attributes.name,
                    id: h.id,
                    region: h.attributes.region.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Hubs(rows))
        }
        ViewKind::ProjectList { hub_id } => {
            let projects = clients.dm.list_projects(hub_id).await?;
            let rows: Vec<ProjectRow> = projects
                .into_iter()
                .map(|p| ProjectRow {
                    name: p.attributes.name,
                    id: p.id,
                })
                .collect();
            Ok(ResourceData::Projects(rows))
        }
        ViewKind::FolderList {
            project_id,
            folder_id,
        } => {
            if let Some(hub_id) = folder_id.strip_prefix("__top__") {
                let folders = clients.dm.get_top_folders(hub_id, project_id).await?;
                let rows: Vec<FolderContentRow> = folders
                    .into_iter()
                    .map(|f| FolderContentRow {
                        name: f.attributes.display_name.unwrap_or(f.attributes.name),
                        content_type: "folder".into(),
                        id: f.id,
                        modified: f.attributes.last_modified_time.unwrap_or_default(),
                    })
                    .collect();
                Ok(ResourceData::FolderContents(rows))
            } else {
                let contents = clients
                    .dm
                    .list_folder_contents(project_id, folder_id)
                    .await?;
                let rows: Vec<FolderContentRow> = contents
                    .into_iter()
                    .filter_map(|val| {
                        let obj = val.as_object()?;
                        let item_type = obj.get("type")?.as_str()?.to_string();
                        let id = obj.get("id")?.as_str()?.to_string();
                        let attrs = obj.get("attributes")?.as_object()?;
                        let name = attrs
                            .get("displayName")
                            .or_else(|| attrs.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("(unknown)")
                            .to_string();
                        let modified = attrs
                            .get("lastModifiedTime")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let content_type = if item_type.contains("folder") {
                            "folder"
                        } else {
                            "item"
                        }
                        .to_string();
                        Some(FolderContentRow {
                            name,
                            content_type,
                            id,
                            modified,
                        })
                    })
                    .collect();
                Ok(ResourceData::FolderContents(rows))
            }
        }
        ViewKind::ItemDetail {
            project_id,
            item_id,
        } => {
            let item = clients.dm.get_item(project_id, item_id).await?;
            let mut fields = vec![
                DetailField { label: "ID".into(), value: item.id },
                DetailField { label: "Name".into(), value: item.attributes.display_name },
                DetailField { label: "Type".into(), value: item.item_type },
                DetailField { label: "Created".into(), value: item.attributes.create_time.unwrap_or_default() },
                DetailField { label: "Modified".into(), value: item.attributes.last_modified_time.unwrap_or_default() },
            ];
            if let Ok(versions) = clients.dm.get_item_versions(project_id, item_id).await {
                fields.push(DetailField {
                    label: "Versions".into(),
                    value: format!("{}", versions.len()),
                });
                for v in versions.iter().take(5) {
                    let ver_num = v
                        .attributes
                        .version_number
                        .map(|n| n.to_string())
                        .unwrap_or_default();
                    let size = v
                        .attributes
                        .storage_size
                        .map(|s| util::format_size(s as u64))
                        .unwrap_or_default();
                    fields.push(DetailField {
                        label: format!("  v{ver_num}"),
                        value: format!(
                            "{} ({})",
                            v.attributes
                                .display_name
                                .as_deref()
                                .unwrap_or(&v.attributes.name),
                            size
                        ),
                    });
                }
            }
            Ok(ResourceData::ItemDetail(fields))
        }
        ViewKind::IssueList { project_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let issues = clients.issues.list_issues(&acc_pid, None).await
                .map_err(|e| anyhow::anyhow!("Issues API (pid={acc_pid}): {e:#}"))?;
            let rows: Vec<IssueRow> = issues
                .into_iter()
                .map(|i| IssueRow {
                    title: i.title,
                    status: i.status,
                    assigned_to: i.assigned_to.unwrap_or_default(),
                    created_at: i.created_at.unwrap_or_default(),
                    id: i.id,
                })
                .collect();
            Ok(ResourceData::Issues(rows))
        }
        ViewKind::IssueDetail {
            project_id,
            issue_id,
        } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let issue = clients.issues.get_issue(&acc_pid, issue_id).await?;
            let fields = vec![
                DetailField { label: "ID".into(), value: issue.id },
                DetailField { label: "Title".into(), value: issue.title },
                DetailField { label: "Status".into(), value: issue.status },
                DetailField { label: "Description".into(), value: issue.description.unwrap_or_default() },
                DetailField { label: "Assigned To".into(), value: issue.assigned_to.unwrap_or_default() },
                DetailField { label: "Due Date".into(), value: issue.due_date.unwrap_or_default() },
                DetailField { label: "Created At".into(), value: issue.created_at.unwrap_or_default() },
                DetailField { label: "Created By".into(), value: issue.created_by.unwrap_or_default() },
            ];
            Ok(ResourceData::IssueDetail(fields))
        }
        ViewKind::IssueCommentList { project_id, issue_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let comments = clients.issues.list_comments(&acc_pid, issue_id).await?;
            let rows: Vec<IssueCommentRow> = comments
                .into_iter()
                .map(|c| IssueCommentRow {
                    id: c.id,
                    body: c.body.chars().take(80).collect(),
                    created_by: c.created_by.unwrap_or_default(),
                    created_at: c.created_at.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::IssueComments(rows))
        }
        ViewKind::IssueAttachmentList { project_id, issue_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let attachments = clients.issues.list_attachments(&acc_pid, issue_id).await?;
            let rows: Vec<IssueAttachmentRow> = attachments
                .into_iter()
                .map(|a| IssueAttachmentRow {
                    id: a.id,
                    name: a.name,
                    urn: a.urn.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::IssueAttachments(rows))
        }
        ViewKind::IssueTypeList { project_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let types = clients.issues.list_issue_types(&acc_pid).await?;
            let rows: Vec<IssueTypeRow> = types
                .into_iter()
                .map(|t| IssueTypeRow {
                    id: t.id,
                    title: t.title,
                    is_active: if t.is_active.unwrap_or(false) { "Yes" } else { "No" }.into(),
                })
                .collect();
            Ok(ResourceData::IssueTypes(rows))
        }
        ViewKind::RfiList { project_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let rfis = clients.rfi.list_rfis(&acc_pid).await
                .map_err(|e| anyhow::anyhow!("RFI API (pid={acc_pid}): {e:#}"))?;
            let rows: Vec<RfiRow> = rfis
                .into_iter()
                .map(|r: Rfi| RfiRow {
                    title: r.title,
                    status: r.status,
                    priority: r.priority.unwrap_or_default(),
                    created_at: r.created_at.unwrap_or_default(),
                    id: r.id,
                })
                .collect();
            Ok(ResourceData::Rfis(rows))
        }
        ViewKind::RfiDetail {
            project_id,
            rfi_id,
        } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let rfi = clients.rfi.get_rfi(&acc_pid, rfi_id).await?;
            let fields = vec![
                DetailField { label: "ID".into(), value: rfi.id },
                DetailField { label: "Title".into(), value: rfi.title },
                DetailField { label: "Status".into(), value: rfi.status },
                DetailField { label: "Priority".into(), value: rfi.priority.unwrap_or_default() },
                DetailField { label: "Question".into(), value: rfi.question.unwrap_or_default() },
                DetailField { label: "Answer".into(), value: rfi.answer.unwrap_or_default() },
                DetailField { label: "Assigned To".into(), value: rfi.assigned_to_name.unwrap_or_default() },
                DetailField { label: "Due Date".into(), value: rfi.due_date.unwrap_or_default() },
                DetailField { label: "Created At".into(), value: rfi.created_at.unwrap_or_default() },
                DetailField { label: "Created By".into(), value: rfi.created_by_name.unwrap_or_default() },
            ];
            Ok(ResourceData::RfiDetail(fields))
        }
        ViewKind::AssetList { project_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let assets = clients.acc.list_assets(&acc_pid).await?;
            let rows: Vec<AssetRow> = assets
                .into_iter()
                .map(|a| AssetRow {
                    id: a.id,
                    client_asset_id: a.client_asset_id.unwrap_or_default(),
                    description: a.description.unwrap_or_default(),
                    status: a.status_id.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Assets(rows))
        }
        ViewKind::AssetDetail { project_id, asset_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let assets = clients.acc.list_assets(&acc_pid).await?;
            let asset = assets.into_iter().find(|a| a.id == *asset_id);
            if let Some(a) = asset {
                let fields = vec![
                    DetailField { label: "ID".into(), value: a.id },
                    DetailField { label: "Client Asset ID".into(), value: a.client_asset_id.unwrap_or_default() },
                    DetailField { label: "Category ID".into(), value: a.category_id.unwrap_or_default() },
                    DetailField { label: "Status ID".into(), value: a.status_id.unwrap_or_default() },
                    DetailField { label: "Description".into(), value: a.description.unwrap_or_default() },
                    DetailField { label: "Barcode".into(), value: a.barcode.unwrap_or_default() },
                    DetailField { label: "Created At".into(), value: a.created_at.unwrap_or_default() },
                    DetailField { label: "Updated At".into(), value: a.updated_at.unwrap_or_default() },
                ];
                Ok(ResourceData::AssetDetail(fields))
            } else {
                anyhow::bail!("Asset {asset_id} not found")
            }
        }
        ViewKind::SubmittalList { project_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let submittals = clients.acc.list_submittals(&acc_pid).await?;
            let rows: Vec<SubmittalRow> = submittals
                .into_iter()
                .map(|s| SubmittalRow {
                    id: s.id,
                    title: s.title,
                    number: s.number.unwrap_or_default(),
                    status: s.status,
                    due_date: s.due_date.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Submittals(rows))
        }
        ViewKind::SubmittalDetail { project_id, submittal_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let submittals = clients.acc.list_submittals(&acc_pid).await?;
            let sub = submittals.into_iter().find(|s| s.id == *submittal_id);
            if let Some(s) = sub {
                let fields = vec![
                    DetailField { label: "ID".into(), value: s.id },
                    DetailField { label: "Title".into(), value: s.title },
                    DetailField { label: "Number".into(), value: s.number.unwrap_or_default() },
                    DetailField { label: "Status".into(), value: s.status },
                    DetailField { label: "Spec Section".into(), value: s.spec_section.unwrap_or_default() },
                    DetailField { label: "Due Date".into(), value: s.due_date.unwrap_or_default() },
                    DetailField { label: "Created At".into(), value: s.created_at.unwrap_or_default() },
                    DetailField { label: "Updated At".into(), value: s.updated_at.unwrap_or_default() },
                ];
                Ok(ResourceData::SubmittalDetail(fields))
            } else {
                anyhow::bail!("Submittal {submittal_id} not found")
            }
        }
        ViewKind::ChecklistList { project_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let checklists = clients.acc.list_checklists(&acc_pid).await?;
            let rows: Vec<ChecklistRow> = checklists
                .into_iter()
                .map(|c| ChecklistRow {
                    id: c.id,
                    title: c.title,
                    status: c.status,
                    location: c.location.unwrap_or_default(),
                    due_date: c.due_date.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Checklists(rows))
        }
        ViewKind::ChecklistDetail { project_id, checklist_id } => {
            let acc_pid = resolve_acc_project_id(clients, project_id, hub_context).await?;
            let checklists = clients.acc.list_checklists(&acc_pid).await?;
            let cl = checklists.into_iter().find(|c| c.id == *checklist_id);
            if let Some(c) = cl {
                let fields = vec![
                    DetailField { label: "ID".into(), value: c.id },
                    DetailField { label: "Title".into(), value: c.title },
                    DetailField { label: "Status".into(), value: c.status },
                    DetailField { label: "Template ID".into(), value: c.template_id.unwrap_or_default() },
                    DetailField { label: "Assignee ID".into(), value: c.assignee_id.unwrap_or_default() },
                    DetailField { label: "Location".into(), value: c.location.unwrap_or_default() },
                    DetailField { label: "Due Date".into(), value: c.due_date.unwrap_or_default() },
                    DetailField { label: "Created At".into(), value: c.created_at.unwrap_or_default() },
                    DetailField { label: "Updated At".into(), value: c.updated_at.unwrap_or_default() },
                ];
                Ok(ResourceData::ChecklistDetail(fields))
            } else {
                anyhow::bail!("Checklist {checklist_id} not found")
            }
        }
        ViewKind::EngineList => {
            let engines = clients.da.list_engines_detailed().await?;
            let rows: Vec<EngineRow> = engines
                .into_iter()
                .map(|e| EngineRow {
                    id: e.id,
                    description: e.description.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Engines(rows))
        }
        ViewKind::ActivityList => {
            let activities = clients.da.list_activities().await?;
            let rows: Vec<ActivityRow> =
                activities.into_iter().map(|id| ActivityRow { id }).collect();
            Ok(ResourceData::Activities(rows))
        }
        ViewKind::WorkItemList => {
            let items = clients.da.list_workitems().await?;
            let rows: Vec<WorkItemRow> = items
                .into_iter()
                .map(|w| WorkItemRow {
                    id: w.id,
                    status: w.status,
                    progress: w.progress.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::WorkItems(rows))
        }
        ViewKind::WorkItemDetail { id } => {
            let wi = clients.da.get_workitem_status(id).await?;
            let mut fields = vec![
                DetailField { label: "ID".into(), value: wi.id },
                DetailField { label: "Status".into(), value: wi.status },
                DetailField { label: "Progress".into(), value: wi.progress.unwrap_or_default() },
                DetailField { label: "Report URL".into(), value: wi.report_url.unwrap_or_default() },
            ];
            if let Some(stats) = wi.stats {
                fields.push(DetailField { label: "Time Queued".into(), value: stats.time_queued.unwrap_or_default() });
                fields.push(DetailField { label: "Download Started".into(), value: stats.time_download_started.unwrap_or_default() });
                fields.push(DetailField { label: "Instruction Started".into(), value: stats.time_instruction_started.unwrap_or_default() });
                fields.push(DetailField { label: "Instruction Ended".into(), value: stats.time_instruction_ended.unwrap_or_default() });
                fields.push(DetailField { label: "Upload Ended".into(), value: stats.time_upload_ended.unwrap_or_default() });
            }
            Ok(ResourceData::WorkItemDetail(fields))
        }
        ViewKind::AppBundleList => {
            let bundles = clients.da.list_appbundles().await?;
            let rows: Vec<AppBundleRow> = bundles
                .into_iter()
                .map(|id| AppBundleRow { id })
                .collect();
            Ok(ResourceData::AppBundles(rows))
        }
        ViewKind::ManifestView { urn } => {
            let manifest = clients.derivative.get_manifest(urn).await?;
            let fields = vec![
                DetailField { label: "URN".into(), value: manifest.urn },
                DetailField { label: "Status".into(), value: manifest.status },
                DetailField { label: "Progress".into(), value: manifest.progress },
                DetailField { label: "Region".into(), value: manifest.region },
                DetailField { label: "Type".into(), value: manifest.manifest_type },
                DetailField { label: "Has Thumbnail".into(), value: manifest.has_thumbnail },
                DetailField { label: "Derivatives".into(), value: format!("{}", manifest.derivatives.len()) },
                DetailField { label: "Version".into(), value: manifest.version.unwrap_or_default() },
            ];
            Ok(ResourceData::Manifest(fields))
        }
        ViewKind::DerivativeList { urn } => {
            let derivs = clients.derivative.list_downloadable_derivatives(urn).await?;
            let rows: Vec<DerivativeRow> = derivs
                .into_iter()
                .map(|d| DerivativeRow {
                    name: d.name,
                    output_type: d.output_type,
                    role: d.role,
                    mime: d.mime.unwrap_or_default(),
                    size: d.size.map(util::format_size).unwrap_or_default(),
                    urn: d.urn,
                })
                .collect();
            Ok(ResourceData::Derivatives(rows))
        }
        ViewKind::DerivativeDetail { urn: _, deriv_urn, name } => {
            // Detail from cached derivative list
            let fields = vec![
                DetailField { label: "Name".into(), value: name.clone() },
                DetailField { label: "URN".into(), value: deriv_urn.clone() },
            ];
            Ok(ResourceData::DerivativeDetail(fields))
        }
        ViewKind::WebhookList => {
            let hooks = clients.webhooks.list_all_webhooks().await?;
            let rows: Vec<WebhookRow> = hooks
                .into_iter()
                .map(|h| WebhookRow {
                    hook_id: h.hook_id,
                    event: h.event,
                    callback_url: h.callback_url,
                    status: h.status,
                    system: h.system,
                    created: h.created_date.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Webhooks(rows))
        }
        ViewKind::WebhookDetail { system, event, hook_id } => {
            let hook = clients.webhooks.get_webhook(system, event, hook_id).await?;
            let fields = vec![
                DetailField { label: "Hook ID".into(), value: hook.hook_id },
                DetailField { label: "System".into(), value: hook.system },
                DetailField { label: "Event".into(), value: hook.event },
                DetailField { label: "Callback URL".into(), value: hook.callback_url },
                DetailField { label: "Status".into(), value: hook.status },
                DetailField { label: "Created By".into(), value: hook.created_by.unwrap_or_default() },
                DetailField { label: "Created".into(), value: hook.created_date.unwrap_or_default() },
                DetailField { label: "Updated".into(), value: hook.last_updated_date.unwrap_or_default() },
                DetailField { label: "Creator Type".into(), value: hook.creator_type.unwrap_or_default() },
                DetailField { label: "Tenant".into(), value: hook.tenant.unwrap_or_default() },
                DetailField { label: "URN".into(), value: hook.urn.unwrap_or_default() },
                DetailField { label: "Auto Reactivate".into(), value: hook.auto_reactivate_hook.map(|b| b.to_string()).unwrap_or_default() },
            ];
            Ok(ResourceData::WebhookDetail(fields))
        }
        ViewKind::PhotosceneList => {
            let scenes = clients.reality.list_photoscenes().await?;
            let rows: Vec<PhotosceneRow> = scenes
                .into_iter()
                .map(|s| PhotosceneRow {
                    id: s.photoscene_id,
                    name: s.name.unwrap_or_default(),
                    scene_type: s.scene_type.unwrap_or_default(),
                    format: s.convert_format.unwrap_or_default(),
                    status: s.status.unwrap_or_default(),
                })
                .collect();
            Ok(ResourceData::Photoscenes(rows))
        }
        ViewKind::PhotosceneDetail { id } => {
            let progress = clients.reality.get_progress(id).await?;
            let fields = vec![
                DetailField { label: "ID".into(), value: progress.photoscene_id },
                DetailField { label: "Progress".into(), value: progress.progress },
                DetailField { label: "Status".into(), value: progress.status.unwrap_or_default() },
                DetailField { label: "Message".into(), value: progress.progress_msg.unwrap_or_default() },
            ];
            Ok(ResourceData::PhotosceneDetail(fields))
        }
    }
}
