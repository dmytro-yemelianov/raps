// SPDX-License-Identifier: Apache-2.0
use super::super::*;
use crate::commands::dashboard::traits::DashboardResource;
use ratatui::{layout::Constraint, widgets::{Cell, Row}};

// --- Derivatives ---

#[derive(Debug, Clone)]
pub struct DerivativeList {
    pub urn: String,
    pub rows: Vec<DerivativeRow>,
}

#[derive(Debug, Clone)]
pub struct DerivativeRow {
    pub name: String,
    pub output_type: String,
    pub role: String,
    pub mime: String,
    pub size: String,
    pub urn: String,
}

impl DashboardResource for DerivativeList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Name", "OutputType", "Role", "MIME", "Size"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() { return self.rows.len(); }
        let filter = filter.to_lowercase();
        self.rows.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&DerivativeRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        
        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.name.as_str()),
                Cell::from(r.output_type.as_str()),
                Cell::from(r.role.as_str()),
                Cell::from(r.mime.as_str()),
                Cell::from(r.size.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&DerivativeRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.urn.clone())
    }

    fn handle_enter(&self, index: usize, filter: &str, app: &mut App, clients: &Arc<Clients>, tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>) {
        if self.get_id(index, filter).is_some() {
            let filter = filter.to_lowercase();
            let filtered: Vec<&DerivativeRow> = self.rows.iter()
                .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
                .collect();
            if let Some(row) = filtered.get(index) {
                app.push_view(ViewKind::DerivativeDetail {
                    urn: self.urn.clone(),
                    deriv_urn: row.urn.clone(),
                    name: row.name.clone(),
                });
                fetch::load_view(app, clients, tx, false);
            }
        }
    }
}

// --- Webhooks ---

#[derive(Debug, Clone)]
pub struct WebhookList {
    pub rows: Vec<WebhookRow>,
}

#[derive(Debug, Clone)]
pub struct WebhookRow {
    pub event: String,
    pub callback_url: String,
    pub status: String,
    pub system: String,
    pub created: String,
    pub hook_id: String,
}

impl DashboardResource for WebhookList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Event", "Callback URL", "Status", "System", "Created"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() { return self.rows.len(); }
        let filter = filter.to_lowercase();
        self.rows.iter().filter(|r| r.event.to_lowercase().contains(&filter) || r.callback_url.to_lowercase().contains(&filter)).count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&WebhookRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.event.to_lowercase().contains(&filter) || r.callback_url.to_lowercase().contains(&filter))
            .collect();
        
        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.event.as_str()),
                Cell::from(r.callback_url.as_str()),
                Cell::from(r.status.as_str()),
                Cell::from(r.system.as_str()),
                Cell::from(r.created.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&WebhookRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.event.to_lowercase().contains(&filter) || r.callback_url.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.hook_id.clone())
    }

    fn handle_enter(&self, index: usize, filter: &str, app: &mut App, clients: &Arc<Clients>, tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>) {
        let filter = filter.to_lowercase();
        let filtered: Vec<&WebhookRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.event.to_lowercase().contains(&filter) || r.callback_url.to_lowercase().contains(&filter))
            .collect();

        if let Some(row) = filtered.get(index) {
            app.push_view(ViewKind::WebhookDetail {
                system: row.system.clone(),
                event: row.event.clone(),
                hook_id: row.hook_id.clone(),
            });
            fetch::load_view(app, clients, tx, false);
        }
    }
}

// --- Photoscenes ---

#[derive(Debug, Clone)]
pub struct PhotosceneList {
    pub rows: Vec<PhotosceneRow>,
}

#[derive(Debug, Clone)]
pub struct PhotosceneRow {
    pub name: String,
    pub id: String,
    pub scene_type: String,
    pub format: String,
    pub status: String,
}

impl DashboardResource for PhotosceneList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Name", "ID", "Type", "Format", "Status"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() { return self.rows.len(); }
        let filter = filter.to_lowercase();
        self.rows.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&PhotosceneRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        
        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.name.as_str()),
                Cell::from(r.id.as_str()),
                Cell::from(r.scene_type.as_str()),
                Cell::from(r.format.as_str()),
                Cell::from(r.status.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&PhotosceneRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(&self, index: usize, filter: &str, app: &mut App, clients: &Arc<Clients>, tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>) {
        if let Some(id) = self.get_id(index, filter) {
            app.push_view(ViewKind::PhotosceneDetail { id });
            fetch::load_view(app, clients, tx, false);
        }
    }
}

// --- Logs ---

#[derive(Debug, Clone)]
pub struct LogList {
    pub rows: Vec<LogRow>,
}

#[derive(Debug, Clone)]
pub struct LogRow {
    pub message: String,
}

impl DashboardResource for LogList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Message"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![Constraint::Percentage(100)]
    }

    fn raw_count(&self) -> usize {
        self.rows.len()
    }

    fn filtered_count(&self, filter: &str) -> usize {
        if filter.is_empty() { return self.rows.len(); }
        let filter = filter.to_lowercase();
        self.rows.iter().filter(|r| r.message.to_lowercase().contains(&filter)).count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&LogRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.message.to_lowercase().contains(&filter))
            .collect();
        
        filtered.get(index).map(|r| {
            Row::new(vec![Cell::from(r.message.as_str())])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&LogRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.message.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.message.clone())
    }

    fn handle_enter(&self, _index: usize, _filter: &str, _app: &mut App, _clients: &Arc<Clients>, _tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>) {
        // No drill-down for logs
    }
}
