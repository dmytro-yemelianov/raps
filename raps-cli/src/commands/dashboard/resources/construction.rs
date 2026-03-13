// SPDX-License-Identifier: Apache-2.0
use super::super::*;
use crate::commands::dashboard::traits::DashboardResource;
use ratatui::{
    layout::Constraint,
    widgets::{Cell, Row},
};

// --- Issues ---

#[derive(Debug, Clone)]
pub struct IssueList {
    pub project_id: String,
    pub rows: Vec<IssueRow>,
}

#[derive(Debug, Clone)]
pub struct IssueRow {
    pub title: String,
    pub status: String,
    pub assigned_to: String,
    pub created_at: String,
    pub id: String,
}

impl DashboardResource for IssueList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Title", "Status", "Assigned", "Created"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(40),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() {
            return self.rows.len();
        }
        let filter = filter.to_lowercase();
        self.rows
            .iter()
            .filter(|r| r.title.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&IssueRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.title.as_str()),
                Cell::from(ratatui::text::Span::styled(
                    r.status.as_str(),
                    util::status_color(&r.status),
                )),
                Cell::from(r.assigned_to.as_str()),
                Cell::from(r.created_at.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&IssueRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        index: usize,
        filter: &str,
        app: &mut App,
        clients: &Arc<Clients>,
        tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        if let Some(id) = self.get_id(index, filter) {
            app.push_view(ViewKind::IssueDetail {
                project_id: self.project_id.clone(),
                issue_id: id,
            });
            fetch::load_view(app, clients, tx, false);
        }
    }
}

// --- RFIs ---

#[derive(Debug, Clone)]
pub struct RfiList {
    pub project_id: String,
    pub rows: Vec<RfiRow>,
}

#[derive(Debug, Clone)]
pub struct RfiRow {
    pub title: String,
    pub status: String,
    pub priority: String,
    pub created_at: String,
    pub id: String,
}

impl DashboardResource for RfiList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Title", "Status", "Priority", "Created"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(40),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(30),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() {
            return self.rows.len();
        }
        let filter = filter.to_lowercase();
        self.rows
            .iter()
            .filter(|r| r.title.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&RfiRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.title.as_str()),
                Cell::from(ratatui::text::Span::styled(
                    r.status.as_str(),
                    util::status_color(&r.status),
                )),
                Cell::from(r.priority.as_str()),
                Cell::from(r.created_at.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&RfiRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        index: usize,
        filter: &str,
        app: &mut App,
        clients: &Arc<Clients>,
        tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        if let Some(id) = self.get_id(index, filter) {
            app.push_view(ViewKind::RfiDetail {
                project_id: self.project_id.clone(),
                rfi_id: id,
            });
            fetch::load_view(app, clients, tx, false);
        }
    }
}

// --- Assets ---

#[derive(Debug, Clone)]
pub struct AssetList {
    pub project_id: String,
    pub rows: Vec<AssetRow>,
}

#[derive(Debug, Clone)]
pub struct AssetRow {
    pub id: String,
    pub client_asset_id: String,
    pub description: String,
    pub status: String,
}

impl DashboardResource for AssetList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["ID", "ClientAssetId", "Description", "Status"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Percentage(35),
            Constraint::Percentage(20),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() {
            return self.rows.len();
        }
        let filter = filter.to_lowercase();
        self.rows
            .iter()
            .filter(|r| {
                r.id.to_lowercase().contains(&filter)
                    || r.description.to_lowercase().contains(&filter)
            })
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&AssetRow> = self
            .rows
            .iter()
            .filter(|r| {
                filter.is_empty()
                    || r.id.to_lowercase().contains(&filter)
                    || r.description.to_lowercase().contains(&filter)
            })
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.id.as_str()),
                Cell::from(r.client_asset_id.as_str()),
                Cell::from(r.description.as_str()),
                Cell::from(r.status.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&AssetRow> = self
            .rows
            .iter()
            .filter(|r| {
                filter.is_empty()
                    || r.id.to_lowercase().contains(&filter)
                    || r.description.to_lowercase().contains(&filter)
            })
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        index: usize,
        filter: &str,
        app: &mut App,
        clients: &Arc<Clients>,
        tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        if let Some(id) = self.get_id(index, filter) {
            app.push_view(ViewKind::AssetDetail {
                project_id: self.project_id.clone(),
                asset_id: id,
            });
            fetch::load_view(app, clients, tx, false);
        }
    }
}

// --- Submittals ---

#[derive(Debug, Clone)]
pub struct SubmittalList {
    pub project_id: String,
    pub rows: Vec<SubmittalRow>,
}

#[derive(Debug, Clone)]
pub struct SubmittalRow {
    pub id: String,
    pub title: String,
    pub number: String,
    pub status: String,
    pub due_date: String,
}

impl DashboardResource for SubmittalList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Title", "Number", "Status", "Due Date"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() {
            return self.rows.len();
        }
        let filter = filter.to_lowercase();
        self.rows
            .iter()
            .filter(|r| r.title.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&SubmittalRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.title.as_str()),
                Cell::from(r.number.as_str()),
                Cell::from(ratatui::text::Span::styled(
                    r.status.as_str(),
                    util::status_color(&r.status),
                )),
                Cell::from(r.due_date.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&SubmittalRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        index: usize,
        filter: &str,
        app: &mut App,
        clients: &Arc<Clients>,
        tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        if let Some(id) = self.get_id(index, filter) {
            app.push_view(ViewKind::SubmittalDetail {
                project_id: self.project_id.clone(),
                submittal_id: id,
            });
            fetch::load_view(app, clients, tx, false);
        }
    }
}

// --- Checklists ---

#[derive(Debug, Clone)]
pub struct ChecklistList {
    pub project_id: String,
    pub rows: Vec<ChecklistRow>,
}

#[derive(Debug, Clone)]
pub struct ChecklistRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub location: String,
    pub due_date: String,
}

impl DashboardResource for ChecklistList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Title", "Status", "Location", "Due Date"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() {
            return self.rows.len();
        }
        let filter = filter.to_lowercase();
        self.rows
            .iter()
            .filter(|r| r.title.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ChecklistRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.title.as_str()),
                Cell::from(ratatui::text::Span::styled(
                    r.status.as_str(),
                    util::status_color(&r.status),
                )),
                Cell::from(r.location.as_str()),
                Cell::from(r.due_date.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ChecklistRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.title.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        index: usize,
        filter: &str,
        app: &mut App,
        clients: &Arc<Clients>,
        tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        if let Some(id) = self.get_id(index, filter) {
            app.push_view(ViewKind::ChecklistDetail {
                project_id: self.project_id.clone(),
                checklist_id: id,
            });
            fetch::load_view(app, clients, tx, false);
        }
    }
}
