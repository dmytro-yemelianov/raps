// SPDX-License-Identifier: Apache-2.0
use super::super::*;
use crate::commands::dashboard::traits::DashboardResource;
use ratatui::{
    layout::Constraint,
    widgets::{Cell, Row},
};

#[derive(Debug, Clone)]
pub struct HubList {
    pub rows: Vec<HubRow>,
}

#[derive(Debug, Clone)]
pub struct HubRow {
    pub name: String,
    pub id: String,
    pub region: String,
}

impl DashboardResource for HubList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Name", "ID", "Region"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(40),
            Constraint::Percentage(40),
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
            .filter(|r| r.name.to_lowercase().contains(&filter))
            .count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&HubRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();

        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.name.as_str()),
                Cell::from(r.id.as_str()),
                Cell::from(r.region.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&HubRow> = self
            .rows
            .iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
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
            app.hub_context = Some(id.clone());
            app.push_view(ViewKind::ProjectList { hub_id: id });
            fetch::load_view(app, clients, tx, false);
        }
    }
}
