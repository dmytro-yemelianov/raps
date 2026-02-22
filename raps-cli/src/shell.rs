// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Interactive shell helper with command completion and hints.
//!
//! Provides tab-completion for RAPS commands and subcommands,
//! as well as inline hints showing required parameters.
//! Uses reedline's Prompt trait for proper styled/raw separation,
//! which fixes cursor alignment issues on Windows.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use reedline::{
    Completer, Highlighter, Hinter, History, Prompt, PromptEditMode, PromptHistorySearch, Span,
    StyledText, Suggestion,
};
// reedline::Prompt uses reedline's own Color type (re-exported from crossterm);
// StyledText uses nu_ansi_term's Style/Color for syntax highlighting.
use nu_ansi_term::{Color as AnsiColor, Style};
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

// ===== Prompt =====

/// RAPS interactive prompt with proper styled/raw separation.
///
/// The Prompt trait separates the raw text content from styling,
/// so reedline can calculate cursor position from plain text width
/// while still displaying a colored prompt.
pub struct RapsPrompt;

impl Prompt for RapsPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("raps> ")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed("(search) ")
    }

    fn get_prompt_color(&self) -> reedline::Color {
        reedline::Color::Yellow
    }
}

// ===== Completer =====

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
            })
            .collect()
    }
}

// ===== Hinter =====

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

// ===== Highlighter =====

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

// ===== Core logic (free functions) =====

/// Get completions for the current input, returning (replacement, description) pairs.
fn get_completions_raw(
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
                        completions
                            .push((subcmd.name.to_string(), subcmd.description.to_string()));
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
                            completions
                                .push((flag_name.to_string(), "(optional)".to_string()));
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
                            completions
                                .push((flag_name.to_string(), "(optional)".to_string()));
                        }
                    }
                }
            }
        }
    }

    completions
}

/// Generate a hint for the current input, returning (display_text, complete_up_to).
fn get_hint_raw(
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
                let positional_count =
                    parts[2..].iter().filter(|p| !p.starts_with('-')).count();

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

/// Build the command tree based on RAPS CLI structure
pub fn build_command_tree() -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            name: "auth",
            description: "Authentication commands",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "login",
                    description: "Authenticate with APS (2-legged or 3-legged)",
                    params: &[],
                    flags: &["--2lo", "--3lo", "--device", "--token <TOKEN>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "logout",
                    description: "Clear stored credentials",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "status",
                    description: "Show current authentication status",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "test",
                    description: "Test authentication by calling API",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "whoami",
                    description: "Show current user profile",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "inspect-token",
                    description: "Inspect current access token",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "bucket",
            description: "Bucket operations (OSS)",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List all buckets",
                    params: &[],
                    flags: &["--limit <N>", "--offset <N>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "create",
                    description: "Create a new bucket",
                    params: &["<BUCKET_KEY>"],
                    flags: &["--retention <transient|temporary|persistent>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get bucket details",
                    params: &["<BUCKET_KEY>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "delete",
                    description: "Delete a bucket",
                    params: &["<BUCKET_KEY>"],
                    flags: &["--force"],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "object",
            description: "Object operations (OSS)",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List objects in a bucket",
                    params: &["<BUCKET_KEY>"],
                    flags: &["--limit <N>", "--prefix <PREFIX>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "upload",
                    description: "Upload a file to a bucket",
                    params: &["<BUCKET_KEY>", "<FILE_PATH>"],
                    flags: &["--key <KEY>", "--batch", "--parallel"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "download",
                    description: "Download an object from a bucket",
                    params: &["<BUCKET_KEY>", "<OBJECT_KEY>"],
                    flags: &["--output <PATH>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "delete",
                    description: "Delete an object from a bucket",
                    params: &["<BUCKET_KEY>", "<OBJECT_KEY>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "details",
                    description: "Get object details",
                    params: &["<BUCKET_KEY>", "<OBJECT_KEY>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "signed-url",
                    description: "Generate signed download URL",
                    params: &["<BUCKET_KEY>", "<OBJECT_KEY>"],
                    flags: &["--minutes <N>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "urn",
                    description: "Get object URN for translation",
                    params: &["<BUCKET_KEY>", "<OBJECT_KEY>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "translate",
            description: "Model Derivative translation",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "start",
                    description: "Start a translation job",
                    params: &["<URN>"],
                    flags: &["--format <svf|svf2>", "--views <2d|3d>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "status",
                    description: "Check translation status",
                    params: &["<URN>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "manifest",
                    description: "Get translation manifest",
                    params: &["<URN>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "metadata",
                    description: "Get model metadata",
                    params: &["<URN>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "tree",
                    description: "Get model hierarchy tree",
                    params: &["<URN>"],
                    flags: &["--guid <GUID>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "properties",
                    description: "Get object properties",
                    params: &["<URN>"],
                    flags: &["--guid <GUID>", "--object-id <ID>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "delete",
                    description: "Delete translation manifest",
                    params: &["<URN>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "hub",
            description: "Hub operations (Data Management)",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List accessible hubs",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get hub details",
                    params: &["<HUB_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "project",
            description: "Project operations (Data Management)",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List projects in a hub",
                    params: &["<HUB_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get project details",
                    params: &["<HUB_ID>", "<PROJECT_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "folder",
            description: "Folder operations (Data Management)",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List folder contents",
                    params: &["<PROJECT_ID>", "<FOLDER_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get folder details",
                    params: &["<PROJECT_ID>", "<FOLDER_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "create",
                    description: "Create a new folder",
                    params: &["<PROJECT_ID>", "<PARENT_FOLDER_ID>", "<NAME>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "item",
            description: "Item/file operations (Data Management)",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "get",
                    description: "Get item details",
                    params: &["<PROJECT_ID>", "<ITEM_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "versions",
                    description: "List item versions",
                    params: &["<PROJECT_ID>", "<ITEM_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "webhook",
            description: "Webhook management",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List webhooks",
                    params: &[],
                    flags: &["--system <SYSTEM>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "create",
                    description: "Create a webhook",
                    params: &["<SYSTEM>", "<EVENT>", "<CALLBACK_URL>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get webhook details",
                    params: &["<SYSTEM>", "<EVENT>", "<HOOK_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "delete",
                    description: "Delete a webhook",
                    params: &["<SYSTEM>", "<EVENT>", "<HOOK_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "da",
            description: "Design Automation",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "engines",
                    description: "List available engines",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "appbundles",
                    description: "List app bundles",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "activities",
                    description: "List activities",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "workitems",
                    description: "List work items",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "issue",
            description: "ACC/BIM 360 Issues",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List issues in a project",
                    params: &["<PROJECT_ID>"],
                    flags: &["--status <STATUS>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get issue details",
                    params: &["<PROJECT_ID>", "<ISSUE_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "create",
                    description: "Create a new issue",
                    params: &["<PROJECT_ID>"],
                    flags: &["--title <TITLE>", "--type <TYPE>"],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "rfi",
            description: "ACC RFIs (Requests for Information)",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List RFIs in a project",
                    params: &["<PROJECT_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get RFI details",
                    params: &["<PROJECT_ID>", "<RFI_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "acc",
            description: "ACC extended modules",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "asset",
                    description: "Manage project assets",
                    params: &[],
                    flags: &[],
                    subcommands: &[
                        CommandInfo {
                            name: "list",
                            description: "List assets in a project",
                            params: &["<PROJECT_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "get",
                            description: "Get a specific asset",
                            params: &["<PROJECT_ID>", "<ASSET_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "create",
                            description: "Create a new asset",
                            params: &["<PROJECT_ID>"],
                            flags: &[
                                "--description <TEXT>",
                                "--barcode <TEXT>",
                                "--category-id <ID>",
                            ],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "update",
                            description: "Update an existing asset",
                            params: &["<PROJECT_ID>", "<ASSET_ID>"],
                            flags: &[
                                "--description <TEXT>",
                                "--barcode <TEXT>",
                                "--status-id <ID>",
                            ],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "delete",
                            description: "Delete an asset",
                            params: &["<PROJECT_ID>", "<ASSET_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                    ],
                },
                CommandInfo {
                    name: "submittal",
                    description: "Manage project submittals",
                    params: &[],
                    flags: &[],
                    subcommands: &[
                        CommandInfo {
                            name: "list",
                            description: "List submittals in a project",
                            params: &["<PROJECT_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "get",
                            description: "Get a specific submittal",
                            params: &["<PROJECT_ID>", "<SUBMITTAL_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "create",
                            description: "Create a new submittal",
                            params: &["<PROJECT_ID>"],
                            flags: &[
                                "--title <TEXT>",
                                "--spec-section <TEXT>",
                                "--due-date <DATE>",
                                "--from-csv <FILE>",
                            ],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "update",
                            description: "Update an existing submittal",
                            params: &["<PROJECT_ID>", "<SUBMITTAL_ID>"],
                            flags: &["--title <TEXT>", "--status <TEXT>", "--due-date <DATE>"],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "delete",
                            description: "Delete a submittal",
                            params: &["<PROJECT_ID>", "<SUBMITTAL_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                    ],
                },
                CommandInfo {
                    name: "checklist",
                    description: "Manage project checklists",
                    params: &[],
                    flags: &[],
                    subcommands: &[
                        CommandInfo {
                            name: "list",
                            description: "List checklists in a project",
                            params: &["<PROJECT_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "get",
                            description: "Get a specific checklist",
                            params: &["<PROJECT_ID>", "<CHECKLIST_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "create",
                            description: "Create a new checklist",
                            params: &["<PROJECT_ID>"],
                            flags: &[
                                "--title <TEXT>",
                                "--template-id <ID>",
                                "--location <TEXT>",
                                "--due-date <DATE>",
                                "--assignee-id <ID>",
                            ],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "update",
                            description: "Update an existing checklist",
                            params: &["<PROJECT_ID>", "<CHECKLIST_ID>"],
                            flags: &[
                                "--title <TEXT>",
                                "--status <TEXT>",
                                "--location <TEXT>",
                                "--due-date <DATE>",
                            ],
                            subcommands: &[],
                        },
                        CommandInfo {
                            name: "templates",
                            description: "List checklist templates",
                            params: &["<PROJECT_ID>"],
                            flags: &[],
                            subcommands: &[],
                        },
                    ],
                },
            ],
        },
        CommandInfo {
            name: "reality",
            description: "Reality Capture / Photogrammetry",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "jobs",
                    description: "List photoscene jobs",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "create",
                    description: "Create a photoscene",
                    params: &["<NAME>"],
                    flags: &["--format <FORMAT>"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "status",
                    description: "Check photoscene status",
                    params: &["<PHOTOSCENE_ID>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "config",
            description: "Configuration management",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "profile",
                    description: "Manage configuration profiles",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "get",
                    description: "Get configuration value",
                    params: &["<KEY>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "set",
                    description: "Set configuration value",
                    params: &["<KEY>", "<VALUE>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "migrate-tokens",
                    description: "Migrate tokens to secure storage",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "pipeline",
            description: "Run pipeline from YAML/JSON file",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "run",
                    description: "Execute a pipeline file",
                    params: &["<FILE>"],
                    flags: &["--dry-run"],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "validate",
                    description: "Validate a pipeline file",
                    params: &["<FILE>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "plugin",
            description: "Plugin management",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "list",
                    description: "List installed plugins",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "add",
                    description: "Add a plugin",
                    params: &["<NAME>", "<PATH>"],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "remove",
                    description: "Remove a plugin",
                    params: &["<NAME>"],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "demo",
            description: "Run demo scenarios",
            params: &[],
            flags: &[],
            subcommands: &[
                CommandInfo {
                    name: "bucket-lifecycle",
                    description: "Demo bucket lifecycle operations",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
                CommandInfo {
                    name: "model-pipeline",
                    description: "Demo model translation pipeline",
                    params: &[],
                    flags: &[],
                    subcommands: &[],
                },
            ],
        },
        CommandInfo {
            name: "generate",
            description: "Generate synthetic engineering files",
            params: &[],
            flags: &["--type <TYPE>", "--output <PATH>"],
            subcommands: &[],
        },
        CommandInfo {
            name: "completions",
            description: "Generate shell completions",
            params: &["<SHELL>"],
            flags: &[],
            subcommands: &[],
        },
        CommandInfo {
            name: "serve",
            description: "Start MCP server for AI integration",
            params: &[],
            flags: &[],
            subcommands: &[],
        },
        CommandInfo {
            name: "help",
            description: "Show help for a command",
            params: &["[COMMAND]"],
            flags: &[],
            subcommands: &[],
        },
        CommandInfo {
            name: "exit",
            description: "Exit the interactive shell",
            params: &[],
            flags: &[],
            subcommands: &[],
        },
        CommandInfo {
            name: "quit",
            description: "Exit the interactive shell",
            params: &[],
            flags: &[],
            subcommands: &[],
        },
    ]
}

/// Build a flat map for quick lookup
fn build_command_map(commands: &[CommandInfo]) -> HashMap<String, CommandInfo> {
    let mut map = HashMap::new();

    for cmd in commands {
        map.insert(cmd.name.to_string(), cmd.clone());

        for subcmd in cmd.subcommands {
            let key = format!("{} {}", cmd.name, subcmd.name);
            map.insert(key, subcmd.clone());
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_tree() {
        let commands = build_command_tree();
        assert!(!commands.is_empty());
        let map = build_command_map(&commands);
        assert!(!map.is_empty());
    }

    #[test]
    fn test_command_completions() {
        let commands = build_command_tree();
        let map = build_command_map(&commands);

        // Test empty line
        let completions = get_completions_raw(&commands, &map, "");
        assert!(!completions.is_empty());

        // Test partial command
        let completions = get_completions_raw(&commands, &map, "au");
        assert!(completions.iter().any(|(r, _)| r == "auth"));

        // Test command with space
        let completions = get_completions_raw(&commands, &map, "auth ");
        assert!(completions.iter().any(|(r, _)| r == "login"));

        // Test partial subcommand
        let completions = get_completions_raw(&commands, &map, "auth log");
        assert!(completions.iter().any(|(r, _)| r == "login"));
        assert!(completions.iter().any(|(r, _)| r == "logout"));
    }

    #[test]
    fn test_hints() {
        let commands = build_command_tree();
        let map = build_command_map(&commands);

        // Test partial command hint
        let hint = get_hint_raw(&commands, &map, "au");
        assert!(hint.is_some());
        let (display, _) = hint.unwrap();
        assert!(display.starts_with("th"));

        // Test complete command shows subcommand hint
        let hint = get_hint_raw(&commands, &map, "auth ");
        assert!(hint.is_some());

        // Test subcommand with params
        let hint = get_hint_raw(&commands, &map, "bucket create ");
        assert!(hint.is_some());
        let (display, _) = hint.unwrap();
        assert!(display.contains("BUCKET_KEY"));
    }
}
