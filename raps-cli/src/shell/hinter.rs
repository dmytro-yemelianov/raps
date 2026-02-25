// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use std::collections::HashMap;

use reedline::{Hinter, History};

use super::CommandInfo;
use super::command_tree::{build_command_map, build_command_tree};

/// Inline hints showing command syntax and required parameters.
pub struct RapsHinter {
    commands: Vec<CommandInfo>,
    command_map: HashMap<String, CommandInfo>,
    /// Completable portion of the current hint (for right-arrow acceptance)
    current_completion: String,
}

impl RapsHinter {
    pub fn new() -> Self {
        let commands = build_command_tree();
        let command_map = build_command_map(&commands);
        Self {
            commands,
            command_map,
            current_completion: String::new(),
        }
    }
}

impl Default for RapsHinter {
    fn default() -> Self {
        Self::new()
    }
}

impl Hinter for RapsHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        _history: &dyn History,
        use_ansi_coloring: bool,
        _cwd: &str,
    ) -> String {
        // Only show hints when cursor is at the end
        if pos < line.len() {
            self.current_completion.clear();
            return String::new();
        }

        match get_hint_raw(&self.commands, &self.command_map, line) {
            Some((display, complete_up_to)) => {
                self.current_completion = if complete_up_to > 0 {
                    display[..complete_up_to].to_string()
                } else {
                    String::new()
                };

                if use_ansi_coloring {
                    // Dim cyan for hint text, matching the old rustyline style
                    format!("\x1b[2;36m{display}\x1b[0m")
                } else {
                    display
                }
            }
            None => {
                self.current_completion.clear();
                String::new()
            }
        }
    }

    fn complete_hint(&self) -> String {
        self.current_completion.clone()
    }

    fn next_hint_token(&self) -> String {
        self.current_completion
            .split_once(' ')
            .map(|(first, _)| first.to_string())
            .unwrap_or_else(|| self.current_completion.clone())
    }
}

/// Generate a hint for the current input, returning (display_text, complete_up_to).
pub(super) fn get_hint_raw(
    commands: &[CommandInfo],
    command_map: &HashMap<String, CommandInfo>,
    line: &str,
) -> Option<(String, usize)> {
    if line.is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    let trailing_space = line.ends_with(' ');

    match parts.len() {
        1 if !trailing_space => {
            // Partial command - find matching command and show full name
            let partial = parts[0].to_lowercase();
            for cmd in commands {
                if cmd.name.starts_with(&partial) && cmd.name != partial {
                    let suffix = &cmd.name[partial.len()..];
                    let mut hint = suffix.to_string();

                    // Add subcommand hint if available
                    if !cmd.subcommands.is_empty() {
                        hint.push_str(" <subcommand>");
                    } else if !cmd.params.is_empty() {
                        hint.push(' ');
                        hint.push_str(&cmd.params.join(" "));
                    }

                    return Some((hint, suffix.len()));
                }
            }
        }
        1 if trailing_space => {
            // Complete command - show subcommands or params
            let cmd_name = parts[0].to_lowercase();
            if let Some(cmd) = commands.iter().find(|c| c.name == cmd_name) {
                if !cmd.subcommands.is_empty() {
                    let subcmd_names: Vec<&str> =
                        cmd.subcommands.iter().take(3).map(|s| s.name).collect();
                    let hint = format!("<{}...>", subcmd_names.join("|"));
                    return Some((hint, 0));
                } else if !cmd.params.is_empty() {
                    let hint = cmd.params.join(" ");
                    return Some((hint, 0));
                }
            }
        }
        2 if !trailing_space => {
            // Partial subcommand
            let cmd_name = parts[0].to_lowercase();
            let partial = parts[1].to_lowercase();

            if let Some(cmd) = commands.iter().find(|c| c.name == cmd_name) {
                for subcmd in cmd.subcommands {
                    if subcmd.name.starts_with(&partial) && subcmd.name != partial {
                        let suffix = &subcmd.name[partial.len()..];
                        let mut hint = suffix.to_string();

                        if !subcmd.params.is_empty() {
                            hint.push(' ');
                            hint.push_str(&subcmd.params.join(" "));
                        }

                        return Some((hint, suffix.len()));
                    }
                }
            }
        }
        2 if trailing_space => {
            // Complete subcommand - show params
            let cmd_name = parts[0].to_lowercase();
            let sub_name = parts[1].to_lowercase();
            let key = format!("{} {}", cmd_name, sub_name);

            if let Some(cmd) = command_map.get(&key) {
                if !cmd.params.is_empty() {
                    let hint = cmd.params.join(" ");
                    return Some((hint, 0));
                } else if !cmd.flags.is_empty() {
                    let hint = format!("[{}]", cmd.flags.first().unwrap_or(&""));
                    return Some((hint, 0));
                }
            }
        }
        n if n >= 3 => {
            // Show remaining params
            let cmd_name = parts[0].to_lowercase();
            let sub_name = parts[1].to_lowercase();
            let key = format!("{} {}", cmd_name, sub_name);

            if let Some(cmd) = command_map.get(&key) {
                // Count how many positional args we have (excluding flags)
                let positional_count = parts[2..].iter().filter(|p| !p.starts_with('-')).count();

                if positional_count < cmd.params.len() {
                    let remaining: Vec<&str> =
                        cmd.params.iter().skip(positional_count).copied().collect();
                    if !remaining.is_empty() && trailing_space {
                        let hint = remaining.join(" ");
                        return Some((hint, 0));
                    }
                }
            }
        }
        _ => {}
    }

    None
}
