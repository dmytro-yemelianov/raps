// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Interactive shell helper with command completion and hints.
//!
//! Provides tab-completion for RAPS commands and subcommands,
//! as well as inline hints showing required parameters.
//! Uses reedline's Prompt trait for proper styled/raw separation,
//! which fixes cursor alignment issues on Windows.

mod command_tree;
mod completer;
mod highlighter;
mod hinter;
mod prompt;
#[cfg(test)]
mod tests;

pub use completer::RapsCompleter;
pub use highlighter::RapsHighlighter;
pub use hinter::RapsHinter;
pub use prompt::RapsPrompt;

use serde::Serialize;

/// Command metadata for completion and hints
#[derive(Debug, Clone, Serialize)]
pub struct CommandInfo {
    /// The command name
    pub name: &'static str,
    /// Short description
    pub description: &'static str,
    /// Required parameters with placeholders (e.g., `<bucket-key>`)
    pub params: &'static [&'static str],
    /// Optional flags
    pub flags: &'static [&'static str],
    /// Subcommands (if any)
    pub subcommands: &'static [CommandInfo],
}
