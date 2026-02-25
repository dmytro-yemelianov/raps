// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use std::collections::HashSet;

use reedline::{Highlighter, StyledText};
// StyledText uses nu_ansi_term's Style/Color for syntax highlighting.
use nu_ansi_term::{Color as AnsiColor, Style};

use super::command_tree::build_command_tree;

/// Colors recognized commands green in the input line.
pub struct RapsHighlighter {
    command_names: HashSet<&'static str>,
}

impl RapsHighlighter {
    pub fn new() -> Self {
        let commands = build_command_tree();
        let command_names = commands.iter().map(|c| c.name).collect();
        Self { command_names }
    }
}

impl Default for RapsHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter for RapsHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();

        if line.is_empty() {
            return styled;
        }

        let first_word_end = line.find(' ').unwrap_or(line.len());
        let cmd = &line[..first_word_end];

        if self.command_names.contains(cmd) {
            styled.push((Style::new().fg(AnsiColor::Green), cmd.to_string()));
        } else {
            styled.push((Style::default(), cmd.to_string()));
        }

        if first_word_end < line.len() {
            styled.push((Style::default(), line[first_word_end..].to_string()));
        }

        styled
    }
}
