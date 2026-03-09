// SPDX-License-Identifier: Apache-2.0
use super::super::*;
use crate::commands::dashboard::traits::DashboardResource;
use ratatui::{layout::Constraint, widgets::{Cell, Row}};

#[derive(Debug, Clone)]
pub struct ObjectList {
    pub bucket_key: String,
    pub rows: Vec<ObjectRow>,
}

#[derive(Debug, Clone)]
pub struct ObjectRow {
    pub key: String,
    pub size: String,
    pub sha1: String,
}

impl DashboardResource for ObjectList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Name", "Size", "SHA1"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(50),
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
        self.rows.iter().filter(|r| r.key.to_lowercase().contains(&filter)).count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ObjectRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.key.to_lowercase().contains(&filter))
            .collect();
        
        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.key.as_str()),
                Cell::from(r.size.as_str()),
                Cell::from(r.sha1.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ObjectRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.key.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.key.clone())
    }

    fn handle_enter(&self, index: usize, filter: &str, app: &mut App, clients: &Arc<Clients>, tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>) {
        if let Some(id) = self.get_id(index, filter) {
            app.push_view(ViewKind::ObjectDetail {
                bucket_key: self.bucket_key.clone(),
                object_key: id,
            });
            fetch::load_view(app, clients, tx, false);
        }
    }
}
