// SPDX-License-Identifier: Apache-2.0
// Trait definitions for dashboard resources

use super::*;
use ratatui::{layout::Constraint, widgets::Row};

pub(super) trait DashboardResource: std::fmt::Debug + Send + Sync {
    /// Column headers for the table
    fn headers(&self) -> Vec<&'static str>;

    /// Column width constraints
    fn widths(&self) -> Vec<Constraint>;

    /// Total number of items (unfiltered)
    fn raw_count(&self) -> usize;

    /// Number of items after filtering
    fn filtered_count(&self, filter: &str) -> usize;

    /// Get the row at the given index after filtering
    fn get_row(&self, index: usize, filter: &str) -> Option<Row<'_>>;

    /// Get the unique ID for the item at the given index after filtering
    fn get_id(&self, index: usize, filter: &str) -> Option<String>;

    /// Action to perform when Enter is pressed on this item
    fn handle_enter(
        &self,
        index: usize,
        filter: &str,
        app: &mut App,
        clients: &Arc<Clients>,
        tx: &tokio::sync::mpsc::UnboundedSender<BackgroundMsg>,
    );

    /// Optional interval for automatic refresh
    fn auto_refresh_interval(&self) -> Option<std::time::Duration> {
        None
    }
}
