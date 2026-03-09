// SPDX-License-Identifier: Apache-2.0
use super::super::*;
use crate::commands::dashboard::traits::DashboardResource;
use ratatui::{layout::Constraint, widgets::{Cell, Row}, style::{Style, Color}, text::Span};

#[derive(Debug, Clone)]
pub struct FolderList {
    pub project_id: String,
    pub rows: Vec<FolderContentRow>,
}

#[derive(Debug, Clone)]
pub struct FolderContentRow {
    pub name: String,
    pub content_type: String,
    pub id: String,
    pub modified: String,
}

impl DashboardResource for FolderList {
    fn headers(&self) -> Vec<&'static str> {
        vec!["Name", "Type", "Modified"]
    }

    fn widths(&self) -> Vec<Constraint> {
        vec![
            Constraint::Percentage(50),
            Constraint::Percentage(15),
            Constraint::Percentage(35),
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
        let filtered: Vec<&FolderContentRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        
        filtered.get(index).map(|r| {
            let type_style = if r.content_type == "folder" {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            let icon = if r.content_type == "folder" { "/" } else { " " };
            Row::new(vec![
                Cell::from(Span::styled(format!("{icon}{}", r.name), type_style)),
                Cell::from(r.content_type.as_str()),
                Cell::from(r.modified.as_str()),
            ])
        })
    }

    fn get_id(&self, index: usize, filter: &str) -> Option<String> {
        let filter = filter.to_lowercase();
        let filtered: Vec<&FolderContentRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();
        filtered.get(index).map(|r| r.id.clone())
    }

    fn handle_enter(&self, index: usize, filter: &str, app: &mut App, clients: &Arc<Clients>, tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>) {
        let filter = filter.to_lowercase();
        let filtered: Vec<&FolderContentRow> = self.rows.iter()
            .filter(|r| filter.is_empty() || r.name.to_lowercase().contains(&filter))
            .collect();

        if let Some(row) = filtered.get(index) {
            if row.content_type == "folder" {
                app.push_view(ViewKind::FolderList {
                    project_id: self.project_id.clone(),
                    folder_id: row.id.clone(),
                });
            } else {
                app.push_view(ViewKind::ItemDetail {
                    project_id: self.project_id.clone(),
                    item_id: row.id.clone(),
                });
            }
            fetch::load_view(app, clients, tx, false);
        }
    }
}
