// SPDX-License-Identifier: Apache-2.0
use super::super::*;
use crate::commands::dashboard::traits::DashboardResource;
use ratatui::{
    layout::Constraint,
    widgets::{Cell, Row},
};

// --- Engines ---

#[derive(Debug, Clone)]
pub struct EngineList {
    pub rows: Vec<EngineRow>,
}

#[derive(Debug, Clone)]
pub struct EngineRow {
    pub id: String,
    pub description: String,
}

impl DashboardResource for EngineList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["ID", "Description"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![Constraint::Percentage(60), Constraint::Percentage(40)]
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
            .filter(|r| r.id.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&EngineRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.id.as_str()),
                Cell::from(r.description.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&EngineRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        _index: usize,
        _filter: &str,
        _app: &mut App,
        _clients: &Arc<Clients>,
        _tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        // No drill-down for engines yet
    }
}

// --- Activities ---

#[derive(Debug, Clone)]
pub struct ActivityList {
    pub rows: Vec<ActivityRow>,
}

#[derive(Debug, Clone)]
pub struct ActivityRow {
    pub id: String,
}

impl DashboardResource for ActivityList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["ID"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![Constraint::Percentage(100)]
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
            .filter(|r| r.id.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ActivityRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
            .collect();

        filtered
            .get(index)
            .map(|r| Row::new(vec![Cell::from(r.id.as_str())]))
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ActivityRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        _index: usize,
        _filter: &str,
        _app: &mut App,
        _clients: &Arc<Clients>,
        _tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        // No drill-down for activities yet
    }
}

// --- WorkItems ---

#[derive(Debug, Clone)]
pub struct WorkItemList {
    pub rows: Vec<WorkItemRow>,
}

#[derive(Debug, Clone)]
pub struct WorkItemRow {
    pub id: String,
    pub status: String,
    pub progress: String,
}

impl DashboardResource for WorkItemList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["ID", "Status", "Progress"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(40),
            Constraint::Percentage(30),
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
            .filter(|r| r.id.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&WorkItemRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.id.as_str()),
                Cell::from(ratatui::text::Span::styled(
                    r.status.as_str(),
                    util::da_status_color(&r.status),
                )),
                Cell::from(r.progress.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&WorkItemRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
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
            app.push_view(ViewKind::WorkItemDetail { id });
            fetch::load_view(app, clients, tx, false);
        }
    }

    fn auto_refresh_interval(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(10))
    }
}

// --- AppBundles ---

#[derive(Debug, Clone)]
pub struct AppBundleList {
    pub rows: Vec<AppBundleRow>,
}

#[derive(Debug, Clone)]
pub struct AppBundleRow {
    pub id: String,
}

impl DashboardResource for AppBundleList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["ID"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![Constraint::Percentage(100)]
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
            .filter(|r| r.id.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&AppBundleRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
            .collect();

        filtered
            .get(index)
            .map(|r| Row::new(vec![Cell::from(r.id.as_str())]))
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&AppBundleRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.id.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(
        &self,
        _index: usize,
        _filter: &str,
        _app: &mut App,
        _clients: &Arc<Clients>,
        _tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    ) {
        // No drill-down for appbundles yet
    }
}
