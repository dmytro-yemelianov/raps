// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use std::collections::HashMap;

use reedline::{Completer, Span, Suggestion};

use super::CommandInfo;
use super::command_tree::{build_command_map, build_command_tree};

/// Tab-completion for RAPS commands, subcommands, and flags.
pub struct RapsCompleter {
    commands: Vec<CommandInfo>,
    command_map: HashMap<String, CommandInfo>,
}

impl RapsCompleter {
    pub fn new() -> Self {
        let commands = build_command_tree();
        let command_map = build_command_map(&commands);
        Self {
            commands,
            command_map,
        }
    }
}

impl Default for RapsCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for RapsCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let input = line.get(..pos).unwrap_or(line);
        let raw = get_completions_raw(&self.commands, &self.command_map, input);

        let start = if input.ends_with(' ') {
            pos
        } else {
            input.rfind(' ').map(|i| i + 1).unwrap_or(0)
        };

        raw.into_iter()
            .map(|(replacement, description)| Suggestion {
                value: replacement,
                description: Some(description),
                style: None,
                extra: None,
                span: Span::new(start, pos),
                append_whitespace: true,
                match_indices: None,
                display_override: None,
            })
            .collect()
    }
}

/// Get completions for the current input, returning (replacement, description) pairs.
pub(super) fn get_completions_raw(
    commands: &[CommandInfo],
    command_map: &HashMap<String, CommandInfo>,
    line: &str,
) -> Vec<(String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let mut completions = Vec::new();

    match parts.len() {
        0 => {
            // Empty line - suggest all top-level commands
            for cmd in commands {
                completions.push((cmd.name.to_string(), cmd.description.to_string()));
            }
        }
        1 => {
            let partial = parts[0].to_lowercase();
            let trailing_space = line.ends_with(' ');

            if trailing_space {
                // Command is complete, suggest subcommands
                if let Some(cmd) = commands.iter().find(|c| c.name == partial) {
                    for subcmd in cmd.subcommands {
                        completions.push((subcmd.name.to_string(), subcmd.description.to_string()));
                    }
                }
            } else {
                // Partial command - filter matching commands
                for cmd in commands {
                    if cmd.name.starts_with(&partial) {
                        completions.push((cmd.name.to_string(), cmd.description.to_string()));
                    }
                }
            }
        }
        2 => {
            let cmd_name = parts[0].to_lowercase();
            let partial = parts[1].to_lowercase();
            let trailing_space = line.ends_with(' ');

            if let Some(cmd) = commands.iter().find(|c| c.name == cmd_name) {
                if trailing_space {
                    // Subcommand is complete, suggest parameters/flags
                    if let Some(subcmd) = cmd.subcommands.iter().find(|s| s.name == partial) {
                        for flag in subcmd.flags {
                            let flag_name = flag.split_whitespace().next().unwrap_or(flag);
                            completions.push((flag_name.to_string(), "(optional)".to_string()));
                        }
                    }
                } else {
                    // Partial subcommand - filter matching subcommands
                    for subcmd in cmd.subcommands {
                        if subcmd.name.starts_with(&partial) {
                            completions
                                .push((subcmd.name.to_string(), subcmd.description.to_string()));
                        }
                    }
                }
            }
        }
        _ => {
            // More than 2 parts - suggest flags
            let cmd_name = parts[0].to_lowercase();
            let sub_name = parts[1].to_lowercase();
            let key = format!("{} {}", cmd_name, sub_name);

            if let Some(cmd) = command_map.get(&key) {
                let last = parts.last().unwrap_or(&"");
                let trailing_space = line.ends_with(' ');

                if trailing_space || last.starts_with('-') {
                    for flag in cmd.flags {
                        let flag_name = flag.split_whitespace().next().unwrap_or(flag);
                        if trailing_space || flag_name.starts_with(last) {
                            completions.push((flag_name.to_string(), "(optional)".to_string()));
                        }
                    }
                }
            }
        }
    }

    completions
}
