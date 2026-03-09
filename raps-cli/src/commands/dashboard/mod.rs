// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! TUI Dashboard — k9s-style resource browser for APS
//!
//! Provides a keyboard-driven terminal interface for browsing APS resources:
//! - Buckets + Objects (OSS)
//! - Hubs > Projects > Folders > Items (Data Management)
//! - Issues + RFIs + Assets + Submittals + Checklists (ACC)
//! - Engines, Activities, Work Items, AppBundles (Design Automation)
//! - Manifests + Derivatives (Model Derivative)
//! - Webhooks
//! - Photoscenes (Reality Capture)

mod fetch;
mod keys;
mod render;
mod util;

use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use tokio::sync::mpsc;

use raps_acc::{AccClient, IssuesClient, Rfi, RfiClient};
use raps_da::DesignAutomationClient;
use raps_derivative::DerivativeClient;
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;
use raps_oss::OssClient;
use raps_reality::RealityCaptureClient;
use raps_webhooks::WebhooksClient;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
enum InputMode {
    Normal,
    Filter(String),
    Command(String),
    Confirm(String),
}

/// Actions that require special handling in the event loop (e.g. leaving TUI).
#[derive(Debug)]
enum PendingAction {
    Login,
    Logout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceTab {
    Buckets,
    DataManagement,
    Issues,
    DesignAutomation,
    ModelDerivative,
    Webhooks,
    RealityCapture,
    Swarm,
}

impl ResourceTab {
    const ALL: [ResourceTab; 8] = [
        ResourceTab::Buckets,
        ResourceTab::DataManagement,
        ResourceTab::Issues,
        ResourceTab::DesignAutomation,
        ResourceTab::ModelDerivative,
        ResourceTab::Webhooks,
        ResourceTab::RealityCapture,
        ResourceTab::Swarm,
    ];

    fn index(self) -> usize {
        match self {
            ResourceTab::Buckets => 0,
            ResourceTab::DataManagement => 1,
            ResourceTab::Issues => 2,
            ResourceTab::DesignAutomation => 3,
            ResourceTab::ModelDerivative => 4,
            ResourceTab::Webhooks => 5,
            ResourceTab::RealityCapture => 6,
            ResourceTab::Swarm => 7,
        }
    }

    fn label(self) -> &'static str {
        match self {
            ResourceTab::Buckets => "F1 Buckets",
            ResourceTab::DataManagement => "F2 Data Mgmt",
            ResourceTab::Issues => "F3 ACC",
            ResourceTab::DesignAutomation => "F4 Design Auto",
            ResourceTab::ModelDerivative => "F5 Derivative",
            ResourceTab::Webhooks => "F6 Webhooks",
            ResourceTab::RealityCapture => "F7 Reality",
            ResourceTab::Swarm => "F8 Swarm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ViewKind {
    // Buckets tab (F1)
    BucketList,
    BucketDetail {
        bucket_key: String,
    },
    ObjectList {
        bucket_key: String,
    },
    ObjectDetail {
        bucket_key: String,
        object_key: String,
    },
    // Data Management tab (F2)
    HubList,
    ProjectList {
        hub_id: String,
    },
    FolderList {
        project_id: String,
        folder_id: String,
    },
    ItemDetail {
        project_id: String,
        item_id: String,
    },
    // ACC tab (F3) — Issues, RFIs, Assets, Submittals, Checklists
    IssueList {
        project_id: String,
    },
    IssueDetail {
        project_id: String,
        issue_id: String,
    },
    IssueCommentList {
        project_id: String,
        issue_id: String,
    },
    IssueAttachmentList {
        project_id: String,
        issue_id: String,
    },
    IssueTypeList {
        project_id: String,
    },
    RfiList {
        project_id: String,
    },
    RfiDetail {
        project_id: String,
        rfi_id: String,
    },
    AssetList {
        project_id: String,
    },
    AssetDetail {
        project_id: String,
        asset_id: String,
    },
    SubmittalList {
        project_id: String,
    },
    SubmittalDetail {
        project_id: String,
        submittal_id: String,
    },
    ChecklistList {
        project_id: String,
    },
    ChecklistDetail {
        project_id: String,
        checklist_id: String,
    },
    // DA tab (F4)
    EngineList,
    ActivityList,
    WorkItemList,
    WorkItemDetail {
        id: String,
    },
    AppBundleList,
    // Model Derivative tab (F5)
    ManifestView {
        urn: String,
    },
    DerivativeList {
        urn: String,
    },
    DerivativeDetail {
        urn: String,
        deriv_urn: String,
        name: String,
    },
    // Webhooks tab (F6)
    WebhookList,
    WebhookDetail {
        system: String,
        event: String,
        hook_id: String,
    },
    // Reality Capture tab (F7)
    PhotosceneList,
    PhotosceneDetail {
        id: String,
    },
    // Swarm tab (F8)
    SwarmOverview,
}

impl ViewKind {
    fn title(&self) -> String {
        match self {
            ViewKind::BucketList => "Buckets".into(),
            ViewKind::BucketDetail { bucket_key } => format!("Bucket: {bucket_key}"),
            ViewKind::ObjectList { bucket_key } => format!("Objects in {bucket_key}"),
            ViewKind::ObjectDetail { object_key, .. } => format!("Object: {object_key}"),
            ViewKind::HubList => "Hubs".into(),
            ViewKind::ProjectList { .. } => "Projects".into(),
            ViewKind::FolderList { .. } => "Folder Contents".into(),
            ViewKind::ItemDetail { .. } => "Item Detail".into(),
            ViewKind::IssueList { .. } => "Issues".into(),
            ViewKind::IssueDetail { .. } => "Issue Detail".into(),
            ViewKind::IssueCommentList { .. } => "Issue Comments".into(),
            ViewKind::IssueAttachmentList { .. } => "Issue Attachments".into(),
            ViewKind::IssueTypeList { .. } => "Issue Types".into(),
            ViewKind::RfiList { .. } => "RFIs".into(),
            ViewKind::RfiDetail { .. } => "RFI Detail".into(),
            ViewKind::AssetList { .. } => "Assets".into(),
            ViewKind::AssetDetail { .. } => "Asset Detail".into(),
            ViewKind::SubmittalList { .. } => "Submittals".into(),
            ViewKind::SubmittalDetail { .. } => "Submittal Detail".into(),
            ViewKind::ChecklistList { .. } => "Checklists".into(),
            ViewKind::ChecklistDetail { .. } => "Checklist Detail".into(),
            ViewKind::EngineList => "Engines".into(),
            ViewKind::ActivityList => "Activities".into(),
            ViewKind::WorkItemList => "Work Items".into(),
            ViewKind::WorkItemDetail { id } => format!("WorkItem: {id}"),
            ViewKind::AppBundleList => "AppBundles".into(),
            ViewKind::ManifestView { urn } => {
                if urn.is_empty() {
                    "Model Derivative".into()
                } else {
                    let short = if urn.len() > 24 {
                        format!("…{}", &urn[urn.len() - 20..])
                    } else {
                        urn.clone()
                    };
                    format!("Manifest: {short}")
                }
            }
            ViewKind::DerivativeList { .. } => "Derivatives".into(),
            ViewKind::DerivativeDetail { name, .. } => format!("Derivative: {name}"),
            ViewKind::WebhookList => "Webhooks".into(),
            ViewKind::WebhookDetail { hook_id, .. } => format!("Webhook: {hook_id}"),
            ViewKind::PhotosceneList => "Photoscenes".into(),
            ViewKind::PhotosceneDetail { id } => format!("Photoscene: {id}"),
            ViewKind::SwarmOverview => "Swarm Status".into(),
        }
    }
}

const NUM_TABS: usize = ResourceTab::ALL.len();

/// Holds table data for each view kind
#[derive(Debug, Clone)]
enum ResourceData {
    Buckets(Vec<BucketRow>),
    Objects(Vec<ObjectRow>),
    BucketDetail(Vec<DetailField>),
    ObjectDetail(Vec<DetailField>),
    Hubs(Vec<HubRow>),
    Projects(Vec<ProjectRow>),
    FolderContents(Vec<FolderContentRow>),
    ItemDetail(Vec<DetailField>),
    Issues(Vec<IssueRow>),
    IssueDetail(Vec<DetailField>),
    IssueComments(Vec<IssueCommentRow>),
    IssueAttachments(Vec<IssueAttachmentRow>),
    IssueTypes(Vec<IssueTypeRow>),
    Rfis(Vec<RfiRow>),
    RfiDetail(Vec<DetailField>),
    Assets(Vec<AssetRow>),
    AssetDetail(Vec<DetailField>),
    Submittals(Vec<SubmittalRow>),
    SubmittalDetail(Vec<DetailField>),
    Checklists(Vec<ChecklistRow>),
    ChecklistDetail(Vec<DetailField>),
    Engines(Vec<EngineRow>),
    Activities(Vec<ActivityRow>),
    WorkItems(Vec<WorkItemRow>),
    WorkItemDetail(Vec<DetailField>),
    AppBundles(Vec<AppBundleRow>),
    Manifest(Vec<DetailField>),
    Derivatives(Vec<DerivativeRow>),
    DerivativeDetail(Vec<DetailField>),
    Webhooks(Vec<WebhookRow>),
    WebhookDetail(Vec<DetailField>),
    Photoscenes(Vec<PhotosceneRow>),
    PhotosceneDetail(Vec<DetailField>),
    SwarmStatus(Vec<DetailField>),
}

impl ResourceData {
    /// Returns the number of rows after applying a filter.
    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() {
            return match self {
                ResourceData::Buckets(v) => v.len(),
                ResourceData::Objects(v) => v.len(),
                ResourceData::Hubs(v) => v.len(),
                ResourceData::Projects(v) => v.len(),
                ResourceData::FolderContents(v) => v.len(),
                ResourceData::Issues(v) => v.len(),
                ResourceData::Rfis(v) => v.len(),
                ResourceData::Assets(v) => v.len(),
                ResourceData::Submittals(v) => v.len(),
                ResourceData::Checklists(v) => v.len(),
                ResourceData::IssueComments(v) => v.len(),
                ResourceData::IssueAttachments(v) => v.len(),
                ResourceData::IssueTypes(v) => v.len(),
                ResourceData::Engines(v) => v.len(),
                ResourceData::Activities(v) => v.len(),
                ResourceData::WorkItems(v) => v.len(),
                ResourceData::AppBundles(v) => v.len(),
                ResourceData::Derivatives(v) => v.len(),
                ResourceData::Webhooks(v) => v.len(),
                ResourceData::Photoscenes(v) => v.len(),
                _ => self.raw_count(),
            };
        }
        let filter = filter.to_lowercase();
        match self {
            ResourceData::Buckets(v) => v.iter().filter(|r| r.key.to_lowercase().contains(&filter)).count(),
            ResourceData::Objects(v) => v.iter().filter(|r| r.key.to_lowercase().contains(&filter)).count(),
            ResourceData::Hubs(v) => v.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count(),
            ResourceData::Projects(v) => v.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count(),
            ResourceData::FolderContents(v) => v.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count(),
            ResourceData::Issues(v) => v.iter().filter(|r| r.title.to_lowercase().contains(&filter)).count(),
            ResourceData::Rfis(v) => v.iter().filter(|r| r.title.to_lowercase().contains(&filter)).count(),
            ResourceData::Assets(v) => v.iter().filter(|r| r.id.to_lowercase().contains(&filter) || r.description.to_lowercase().contains(&filter)).count(),
            ResourceData::Submittals(v) => v.iter().filter(|r| r.title.to_lowercase().contains(&filter)).count(),
            ResourceData::Checklists(v) => v.iter().filter(|r| r.title.to_lowercase().contains(&filter)).count(),
            ResourceData::IssueComments(v) => v.iter().filter(|r| r.body.to_lowercase().contains(&filter)).count(),
            ResourceData::IssueAttachments(v) => v.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count(),
            ResourceData::IssueTypes(v) => v.iter().filter(|r| r.title.to_lowercase().contains(&filter)).count(),
            ResourceData::Engines(v) => v.iter().filter(|r| r.id.to_lowercase().contains(&filter)).count(),
            ResourceData::Activities(v) => v.iter().filter(|r| r.id.to_lowercase().contains(&filter)).count(),
            ResourceData::WorkItems(v) => v.iter().filter(|r| r.id.to_lowercase().contains(&filter)).count(),
            ResourceData::AppBundles(v) => v.iter().filter(|r| r.id.to_lowercase().contains(&filter)).count(),
            ResourceData::Derivatives(v) => v.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count(),
            ResourceData::Webhooks(v) => v.iter().filter(|r| r.event.to_lowercase().contains(&filter) || r.callback_url.to_lowercase().contains(&filter)).count(),
            ResourceData::Photoscenes(v) => v.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count(),
            // Detail views (also filterable now for better UX)
            ResourceData::BucketDetail(v)
            | ResourceData::ObjectDetail(v)
            | ResourceData::ItemDetail(v)
            | ResourceData::IssueDetail(v)
            | ResourceData::RfiDetail(v)
            | ResourceData::AssetDetail(v)
            | ResourceData::SubmittalDetail(v)
            | ResourceData::ChecklistDetail(v)
            | ResourceData::WorkItemDetail(v)
            | ResourceData::Manifest(v)
            | ResourceData::DerivativeDetail(v)
            | ResourceData::WebhookDetail(v)
            | ResourceData::PhotosceneDetail(v)
            | ResourceData::SwarmStatus(v) => v.iter().filter(|f| f.label.to_lowercase().contains(&filter) || f.value.to_lowercase().contains(&filter)).count(),
        }
    }

    fn raw_count(&self) -> usize {
        match self {
            ResourceData::Buckets(v) => v.len(),
            ResourceData::Objects(v) => v.len(),
            ResourceData::BucketDetail(v) => v.len(),
            ResourceData::ObjectDetail(v) => v.len(),
            ResourceData::Hubs(v) => v.len(),
            ResourceData::Projects(v) => v.len(),
            ResourceData::FolderContents(v) => v.len(),
            ResourceData::ItemDetail(v) => v.len(),
            ResourceData::Issues(v) => v.len(),
            ResourceData::IssueDetail(v) => v.len(),
            ResourceData::IssueComments(v) => v.len(),
            ResourceData::IssueAttachments(v) => v.len(),
            ResourceData::IssueTypes(v) => v.len(),
            ResourceData::Rfis(v) => v.len(),
            ResourceData::RfiDetail(v) => v.len(),
            ResourceData::Assets(v) => v.len(),
            ResourceData::AssetDetail(v) => v.len(),
            ResourceData::Submittals(v) => v.len(),
            ResourceData::SubmittalDetail(v) => v.len(),
            ResourceData::Checklists(v) => v.len(),
            ResourceData::ChecklistDetail(v) => v.len(),
            ResourceData::Engines(v) => v.len(),
            ResourceData::Activities(v) => v.len(),
            ResourceData::WorkItems(v) => v.len(),
            ResourceData::WorkItemDetail(v) => v.len(),
            ResourceData::AppBundles(v) => v.len(),
            ResourceData::Manifest(v) => v.len(),
            ResourceData::Derivatives(v) => v.len(),
            ResourceData::DerivativeDetail(v) => v.len(),
            ResourceData::Webhooks(v) => v.len(),
            ResourceData::WebhookDetail(v) => v.len(),
            ResourceData::Photoscenes(v) => v.len(),
            ResourceData::PhotosceneDetail(v) => v.len(),
            ResourceData::SwarmStatus(v) => v.len(),
        }
    }
}

// --- Row structs ---

#[derive(Debug, Clone)]
struct BucketRow {
    key: String,
    policy: String,
    created: String,
}

#[derive(Debug, Clone)]
struct ObjectRow {
    key: String,
    size: String,
    sha1: String,
}

#[derive(Debug, Clone)]
struct HubRow {
    name: String,
    id: String,
    region: String,
}

#[derive(Debug, Clone)]
struct ProjectRow {
    name: String,
    id: String,
}

#[derive(Debug, Clone)]
struct FolderContentRow {
    name: String,
    content_type: String,
    id: String,
    modified: String,
}

#[derive(Debug, Clone)]
struct IssueRow {
    title: String,
    status: String,
    assigned_to: String,
    created_at: String,
    id: String,
}

#[derive(Debug, Clone)]
struct RfiRow {
    title: String,
    status: String,
    priority: String,
    created_at: String,
    id: String,
}

#[derive(Debug, Clone)]
struct AssetRow {
    id: String,
    client_asset_id: String,
    description: String,
    status: String,
}

#[derive(Debug, Clone)]
struct SubmittalRow {
    id: String,
    title: String,
    number: String,
    status: String,
    due_date: String,
}

#[derive(Debug, Clone)]
struct ChecklistRow {
    id: String,
    title: String,
    status: String,
    location: String,
    due_date: String,
}

#[derive(Debug, Clone)]
struct IssueCommentRow {
    id: String,
    body: String,
    created_by: String,
    created_at: String,
}

#[derive(Debug, Clone)]
struct IssueAttachmentRow {
    id: String,
    name: String,
    urn: String,
}

#[derive(Debug, Clone)]
struct IssueTypeRow {
    id: String,
    title: String,
    is_active: String,
}

#[derive(Debug, Clone)]
struct EngineRow {
    id: String,
    description: String,
}

#[derive(Debug, Clone)]
struct ActivityRow {
    id: String,
}

#[derive(Debug, Clone)]
struct WorkItemRow {
    id: String,
    status: String,
    progress: String,
}

#[derive(Debug, Clone)]
struct AppBundleRow {
    id: String,
}

#[derive(Debug, Clone)]
struct DerivativeRow {
    name: String,
    output_type: String,
    role: String,
    mime: String,
    size: String,
    urn: String,
}

#[derive(Debug, Clone)]
struct WebhookRow {
    hook_id: String,
    event: String,
    callback_url: String,
    status: String,
    system: String,
    created: String,
}

#[derive(Debug, Clone)]
struct PhotosceneRow {
    id: String,
    name: String,
    scene_type: String,
    format: String,
    status: String,
}

#[derive(Debug, Clone)]
struct DetailField {
    label: String,
    value: String,
}

/// Messages from background fetch tasks
enum BackgroundMsg {
    DataReady(ViewKind, ResourceData),
    Error(String),
    Log(String),
}

/// Cache TTL — cached data older than this is considered stale and refetched.
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Minimum interval between API calls to avoid overload.
const API_THROTTLE: Duration = Duration::from_secs(2);

/// How long a status message stays visible before fading to shortcut hints.
const STATUS_MSG_TTL: Duration = Duration::from_secs(5);

/// A cached view: data + metadata for restoring state when navigating back.
#[derive(Clone)]
struct CacheEntry {
    data: ResourceData,
    fetched_at: Instant,
    /// Saved table selection so we restore cursor position on back-navigation.
    table_selection: Option<usize>,
}

impl CacheEntry {
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < CACHE_TTL
    }
}

/// All API clients bundled
#[derive(Clone)]
struct Clients {
    auth: AuthClient,
    oss: OssClient,
    dm: DataManagementClient,
    issues: IssuesClient,
    rfi: RfiClient,
    da: DesignAutomationClient,
    acc: AccClient,
    derivative: DerivativeClient,
    webhooks: WebhooksClient,
    reality: RealityCaptureClient,
}

/// Application state
struct App {
    tab: ResourceTab,
    nav_stacks: [Vec<ViewKind>; NUM_TABS],
    table_state: TableState,
    /// Currently displayed data for the active view (set from cache or fresh fetch).
    data: Option<ResourceData>,
    loading: bool,
    input_mode: InputMode,
    filter_text: String,
    error_msg: Option<String>,
    status_msg: String,
    /// When the status_msg was last set — shown for STATUS_MSG_TTL then fades to hints
    status_at: Option<Instant>,
    last_refresh: Option<Instant>,
    project_context: Option<(String, String)>,
    hub_context: Option<String>,
    quit: bool,
    /// Per-view data cache: avoids refetching on tab switch / back-navigation.
    cache: HashMap<ViewKind, CacheEntry>,
    /// API call counter for usage display
    api_calls: u32,
    /// Log messages ring buffer (newest last, max 50)
    logs: VecDeque<String>,
    /// Last time a fetch was dispatched (throttling)
    last_fetch: Option<Instant>,
    /// Whether 3-legged auth is active
    logged_in: bool,
    /// Action that needs event-loop-level handling (e.g. leaving TUI for login)
    pending_action: Option<PendingAction>,
    /// Frame counter for spinner animation
    tick: u64,
}

impl App {
    fn new() -> Self {
        Self {
            tab: ResourceTab::Buckets,
            nav_stacks: [
                vec![ViewKind::BucketList],     // Tab 0: Buckets
                vec![ViewKind::HubList],        // Tab 1: Data Mgmt
                vec![ViewKind::HubList],        // Tab 2: ACC (navigate Hub > Project > Issues)
                vec![ViewKind::EngineList],     // Tab 3: Design Automation
                vec![ViewKind::ManifestView { urn: String::new() }], // Tab 4: Model Derivative (avoid empty stack)
                vec![ViewKind::WebhookList],    // Tab 5: Webhooks
                vec![ViewKind::PhotosceneList], // Tab 6: Reality Capture
                vec![ViewKind::SwarmOverview],  // Tab 7: Swarm
            ],
            table_state: TableState::default(),
            data: None,
            loading: false,
            input_mode: InputMode::Normal,
            filter_text: String::new(),
            error_msg: None,
            status_msg: String::new(),
            status_at: None,
            last_refresh: None,
            project_context: None,
            hub_context: None,
            quit: false,
            cache: HashMap::new(),
            api_calls: 0,
            logs: VecDeque::new(),
            last_fetch: None,
            logged_in: false,
            pending_action: None,
            tick: 0,
        }
    }

    fn current_view(&self) -> &ViewKind {
        let stack = &self.nav_stacks[self.tab.index()];
        stack.last().expect("nav stack should never be empty")
    }

    /// Returns true if the current tab's nav stack is non-empty and has valid context.
    fn has_current_view(&self) -> bool {
        let stack = &self.nav_stacks[self.tab.index()];
        if let Some(ViewKind::ManifestView { urn }) = stack.last() {
            !urn.is_empty()
        } else {
            !stack.is_empty()
        }
    }

    /// Save current table selection into the cache entry for the current view.
    fn save_selection_to_cache(&mut self) {
        if !self.has_current_view() {
            return;
        }
        let view = self.current_view().clone();
        if let Some(entry) = self.cache.get_mut(&view) {
            entry.table_selection = self.table_state.selected();
        }
    }

    fn push_view(&mut self, view: ViewKind) {
        // Save cursor position for the view we're leaving
        self.save_selection_to_cache();
        self.nav_stacks[self.tab.index()].push(view);
        self.table_state.select(Some(0));
        self.data = None;
        self.error_msg = None;
    }

    fn pop_view(&mut self) -> bool {
        let stack = &mut self.nav_stacks[self.tab.index()];
        if stack.len() > 1 {
            stack.pop();
            self.error_msg = None;
            // Restore from cache if available
            let view = stack
                .last()
                .expect("navigation stack is never empty")
                .clone();
            if let Some(entry) = self.cache.get(&view) {
                self.data = Some(entry.data.clone());
                self.last_refresh = Some(entry.fetched_at);
                self.table_state.select(entry.table_selection.or(Some(0)));
                let count = self.row_count();
                self.status_msg = format!("{count} items (cached)");
            } else {
                self.data = None;
                self.table_state.select(Some(0));
            }
            true
        } else {
            false
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
        self.status_at = Some(Instant::now());
    }

    /// Whether the status message is still fresh (within TTL).
    fn status_is_active(&self) -> bool {
        self.status_at.is_some_and(|t| t.elapsed() < STATUS_MSG_TTL)
    }

    fn push_log(&mut self, msg: String) {
        if self.logs.len() >= 50 {
            self.logs.pop_front();
        }
        self.logs.push_back(msg);
    }

    fn breadcrumb(&self) -> String {
        let stack = &self.nav_stacks[self.tab.index()];
        if !self.has_current_view() {
            return "Use :urn <base64> to set context".into();
        }
        stack
            .iter()
            .map(|v| v.title())
            .collect::<Vec<_>>()
            .join(" > ")
    }

    fn row_count(&self) -> usize {
        self.data.as_ref().map(|d| d.filtered_count(&self.filter_text)).unwrap_or(0)
    }
}

// ============================================================================
// Entry point
// ============================================================================

pub async fn run_dashboard(config: Config, http_config: HttpClientConfig) -> Result<()> {
    let auth = AuthClient::new_with_http_config(config.clone(), http_config.clone())?;
    let clients = Arc::new(Clients {
        auth: auth.clone(),
        oss: OssClient::new_with_http_config(config.clone(), auth.clone(), http_config.clone())?,
        dm: DataManagementClient::new_with_http_config(
            config.clone(),
            auth.clone(),
            http_config.clone(),
        )?,
        issues: IssuesClient::new_with_http_config(
            config.clone(),
            auth.clone(),
            http_config.clone(),
        )?,
        rfi: RfiClient::new_with_http_config(config.clone(), auth.clone(), http_config.clone())?,
        da: DesignAutomationClient::new_with_http_config(
            config.clone(),
            auth.clone(),
            http_config.clone(),
        )?,
        acc: AccClient::new_with_http_config(config.clone(), auth.clone(), http_config.clone())?,
        derivative: DerivativeClient::new_with_http_config(
            config.clone(),
            auth.clone(),
            http_config.clone(),
        )?,
        webhooks: WebhooksClient::new_with_http_config(
            config.clone(),
            auth.clone(),
            http_config.clone(),
        )?,
        reality: RealityCaptureClient::new_with_http_config(config, auth, http_config)?,
    });

    // Guard restores terminal on both normal exit and panic
    struct TerminalGuard {
        active: bool,
    }
    impl Drop for TerminalGuard {
        fn drop(&mut self) {
            if self.active {
                let _ = disable_raw_mode();
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
            }
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut guard = TerminalGuard { active: true };
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, clients).await;

    // Normal cleanup — deactivate guard so it doesn't double-restore
    guard.active = false;
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    clients: Arc<Clients>,
) -> Result<()> {
    let mut app = App::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<BackgroundMsg>();

    // Check login state on startup
    app.logged_in = clients.auth.is_logged_in().await;
    if !app.logged_in {
        app.push_log("Not logged in. 2-legged auth (client credentials) will be used.".into());
        app.push_log("For 3-legged features, run: raps auth login".into());
    } else {
        app.push_log("Logged in (3-legged auth active)".into());
    }

    fetch::load_view(&mut app, &clients, &tx, false);

    loop {
        app.tick = app.tick.wrapping_add(1);
        terminal.draw(|f| render::ui(f, &mut app))?;

        if app.quit {
            break;
        }

        // Drain background messages first (non-blocking)
        while let Ok(msg) = rx.try_recv() {
            fetch::handle_bg_msg(&mut app, msg);
        }

        // Poll for key events with a short timeout
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == event::KeyEventKind::Press
        {
            keys::handle_key(&mut app, key, &clients, &tx);
        }

        // Handle pending actions that require event-loop-level control
        if let Some(action) = app.pending_action.take() {
            match action {
                PendingAction::Login => {
                    // Leave TUI so the browser can open and user sees progress
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;

                    let scopes: &[&str] = &[
                        "data:read",
                        "data:write",
                        "data:create",
                        "data:search",
                        "bucket:create",
                        "bucket:read",
                        "account:read",
                        "user:read",
                        "viewables:read",
                    ];
                    match clients.auth.login(scopes).await {
                        Ok(_) => {
                            app.logged_in = true;
                            app.cache.clear();
                            app.push_log("Login successful (3-legged auth)".into());
                            app.set_status("Logged in — press r to refresh");
                        }
                        Err(e) => {
                            app.push_log(format!("Login failed: {e:#}"));
                            app.error_msg = Some(format!("Login failed: {e:#}"));
                        }
                    }

                    // Re-enter TUI
                    enable_raw_mode()?;
                    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                    terminal.hide_cursor()?;
                    terminal.clear()?;

                    // Reload current view with new auth
                    fetch::load_view(&mut app, &clients, &tx, true);
                }
                PendingAction::Logout => {
                    match clients.auth.logout().await {
                        Ok(()) => {
                            app.logged_in = false;
                            app.cache.clear();
                            app.push_log("Logged out".into());
                            app.set_status("Logged out (2-legged auth)");
                            // Reload current view with 2-leg auth
                            fetch::load_view(&mut app, &clients, &tx, true);
                        }
                        Err(e) => {
                            app.push_log(format!("Logout failed: {e:#}"));
                            app.error_msg = Some(format!("Logout failed: {e:#}"));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
