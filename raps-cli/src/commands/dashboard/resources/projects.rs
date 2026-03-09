// SPDX-License-Identifier: Apache-2.0
use super::super::*;
use crate::commands::dashboard::traits::DashboardResource;
use ratatui::{layout::Constraint, widgets::{Cell, Row}};

#[derive(Debug, Clone)]
pub struct ProjectList {
    pub hub_id: String,
    pub rows: Vec<ProjectRow>,
}

#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub name: String,
    pub id: String,
}

impl DashboardResource for ProjectList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Name", "ID"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(50),
            Constraint::Percentage(50),
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
        self.rows.iter().filter(|r| r.name.to_lowercase().contains(&filter)).count()
    }

    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ProjectRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        
        filtered.get(index).map(|r| {
            Row::new(vec![
                Cell::from(r.name.as_str()),
                Cell::from(r.id.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ProjectRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(&self, index: usize, filter: &str, app: &mut App, clients: &Arc<Clients>, tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>) {
        let filter = filter.to_lowercase();
        let filtered: Vec<&ProjectRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();

        if let Some(row) = filtered.get(index) {
            let project_id = row.id.clone();
            let project_name = row.name.clone();
            app.project_context = Some((project_id.clone(), project_name));
            if app.tab == ResourceTab::Issues {
                app.push_view(ViewKind::IssueList { project_id });
                fetch::load_view(app, clients, tx, false);
            } else {
                app.push_view(ViewKind::FolderList {
                    project_id,
                    folder_id: format!("__top__{}", self.hub_id),
                });
                fetch::load_view(app, clients, tx, false);
            }
        }
    }
}
