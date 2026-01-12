// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Progress bar and spinner utilities
//!
//! Provides centralized progress bar creation with automatic handling of
//! non-interactive mode. Progress bars are hidden when running in CI/CD
//! or when output is piped.

use indicatif::{ProgressBar, ProgressStyle};

use crate::interactive;

/// Standard progress bar style for file operations (upload/download)
const FILE_PROGRESS_TEMPLATE: &str =
    "{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)";

/// Standard spinner style for async operations
const SPINNER_TEMPLATE: &str = "{spinner:.cyan} {msg}";

/// Progress bar characters
const PROGRESS_CHARS: &str = "█▓░";

/// Create a progress bar for file operations (upload/download)
///
/// Automatically hides the progress bar in non-interactive mode.
pub fn file_progress(size: u64, message: &str) -> ProgressBar {
    let pb = if interactive::is_non_interactive() {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(size)
    };

    pb.set_style(
        ProgressStyle::default_bar()
            .template(FILE_PROGRESS_TEMPLATE)
            .unwrap()
            .progress_chars(PROGRESS_CHARS),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a spinner for async/waiting operations
///
/// Automatically hides the spinner in non-interactive mode.
pub fn spinner(message: &str) -> ProgressBar {
    let pb = if interactive::is_non_interactive() {
        ProgressBar::hidden()
    } else {
        ProgressBar::new_spinner()
    };

    pb.set_style(
        ProgressStyle::default_spinner()
            .template(SPINNER_TEMPLATE)
            .unwrap(),
    );
    pb.set_message(message.to_string());

    if !interactive::is_non_interactive() {
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
    }

    pb
}

/// Create a progress bar for counting items
///
/// Automatically hides the progress bar in non-interactive mode.
pub fn item_progress(count: u64, message: &str) -> ProgressBar {
    let pb = if interactive::is_non_interactive() {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(count)
    };

    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars(PROGRESS_CHARS),
    );
    pb.set_message(message.to_string());
    pb
}

/// Progress bar guard that ensures proper cleanup on drop
///
/// Use this when there's a risk of early return or panic during progress operations.
pub struct ProgressGuard {
    pb: ProgressBar,
    abandon_on_drop: bool,
}

impl ProgressGuard {
    /// Create a new progress guard wrapping a progress bar
    pub fn new(pb: ProgressBar) -> Self {
        Self {
            pb,
            abandon_on_drop: true,
        }
    }

    /// Mark the progress as successfully completed
    ///
    /// Call this when the operation completes successfully to prevent
    /// the guard from abandoning the progress bar on drop.
    pub fn finish(mut self, message: &str) {
        self.abandon_on_drop = false;
        self.pb.finish_with_message(message.to_string());
    }

    /// Get a reference to the underlying progress bar
    pub fn progress(&self) -> &ProgressBar {
        &self.pb
    }
}

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        if self.abandon_on_drop {
            self.pb.abandon();
        }
    }
}

impl std::ops::Deref for ProgressGuard {
    type Target = ProgressBar;

    fn deref(&self) -> &Self::Target {
        &self.pb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_interactive_state() {
        interactive::init(false, false);
    }

    fn set_non_interactive() {
        interactive::init(true, false);
    }

    // ==================== File Progress Tests ====================

    #[test]
    fn test_file_progress_creation() {
        reset_interactive_state();
        let pb = file_progress(1000, "Uploading");
        assert_eq!(pb.length(), Some(1000));
    }

    #[test]
    fn test_file_progress_zero_size() {
        reset_interactive_state();
        let pb = file_progress(0, "Uploading empty file");
        assert_eq!(pb.length(), Some(0));
    }

    #[test]
    fn test_file_progress_large_size() {
        reset_interactive_state();
        let pb = file_progress(u64::MAX, "Uploading huge file");
        assert_eq!(pb.length(), Some(u64::MAX));
    }

    #[test]
    fn test_file_progress_non_interactive() {
        set_non_interactive();
        let pb = file_progress(1000, "Uploading");
        // In non-interactive mode, progress bar is hidden
        // Hidden progress bars return None for length
        assert!(pb.length().is_none());
        reset_interactive_state();
    }

    // ==================== Spinner Tests ====================

    #[test]
    fn test_spinner_creation() {
        reset_interactive_state();
        let pb = spinner("Processing...");
        assert!(pb.length().is_none()); // Spinners have no length
    }

    #[test]
    fn test_spinner_non_interactive() {
        set_non_interactive();
        let pb = spinner("Processing...");
        assert!(pb.length().is_none());
        reset_interactive_state();
    }

    #[test]
    fn test_spinner_empty_message() {
        reset_interactive_state();
        let pb = spinner("");
        assert!(pb.length().is_none());
    }

    // ==================== Item Progress Tests ====================

    #[test]
    fn test_item_progress_creation() {
        reset_interactive_state();
        let pb = item_progress(10, "Processing items");
        assert_eq!(pb.length(), Some(10));
    }

    #[test]
    fn test_item_progress_single_item() {
        reset_interactive_state();
        let pb = item_progress(1, "Processing item");
        assert_eq!(pb.length(), Some(1));
    }

    #[test]
    fn test_item_progress_zero_items() {
        reset_interactive_state();
        let pb = item_progress(0, "No items");
        assert_eq!(pb.length(), Some(0));
    }

    #[test]
    fn test_item_progress_non_interactive() {
        set_non_interactive();
        let pb = item_progress(10, "Processing items");
        // Hidden progress bars return None for length
        assert!(pb.length().is_none());
        reset_interactive_state();
    }

    // ==================== Progress Guard Tests ====================

    #[test]
    fn test_progress_guard_finish() {
        reset_interactive_state();
        let pb = file_progress(100, "Test");
        let guard = ProgressGuard::new(pb);
        guard.finish("Done");
        // No panic on drop
    }

    #[test]
    fn test_progress_guard_abandon_on_drop() {
        reset_interactive_state();
        let pb = file_progress(100, "Test");
        let _guard = ProgressGuard::new(pb);
        // Guard will abandon on drop - this shouldn't panic
    }

    #[test]
    fn test_progress_guard_deref() {
        reset_interactive_state();
        let pb = file_progress(100, "Test");
        let guard = ProgressGuard::new(pb);
        // Can use deref to access progress bar methods
        assert_eq!(guard.length(), Some(100));
    }

    #[test]
    fn test_progress_guard_progress_method() {
        reset_interactive_state();
        let pb = file_progress(100, "Test");
        let guard = ProgressGuard::new(pb);
        // Can access underlying progress bar
        assert_eq!(guard.progress().length(), Some(100));
    }

    #[test]
    fn test_progress_guard_increment() {
        reset_interactive_state();
        let pb = file_progress(100, "Test");
        let guard = ProgressGuard::new(pb);
        guard.inc(50);
        assert_eq!(guard.position(), 50);
    }

    #[test]
    fn test_progress_guard_set_position() {
        reset_interactive_state();
        let pb = file_progress(100, "Test");
        let guard = ProgressGuard::new(pb);
        guard.set_position(75);
        assert_eq!(guard.position(), 75);
    }

    // ==================== Constants Tests ====================

    #[test]
    fn test_progress_chars_length() {
        // "█▓░" - 3 UTF-8 characters, 9 bytes total (3 bytes each)
        assert_eq!(PROGRESS_CHARS.len(), 9);
        assert_eq!(PROGRESS_CHARS.chars().count(), 3);
    }

    #[test]
    fn test_file_template_contains_bar() {
        assert!(FILE_PROGRESS_TEMPLATE.contains("{bar:"));
    }

    #[test]
    fn test_spinner_template_contains_spinner() {
        assert!(SPINNER_TEMPLATE.contains("{spinner"));
    }
}
