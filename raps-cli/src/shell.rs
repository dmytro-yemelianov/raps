// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Interactive shell helper with command completion and hints.
//!
//! Provides tab-completion for RAPS commands and subcommands,
//! as well as inline hints showing required parameters.

use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::HashMap;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hint, Hinter};
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

/// Command metadata for completion and hints
#[derive(Debug, Clone)]
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

/// Custom hint that shows command syntax
#[derive(Debug, Clone)]
pub struct CommandHint {
    /// The full hint text to display (grayed out)
    display: String,
    /// Text to insert when completing (pressing right arrow)
    complete_up_to: usize,
}

impl Hint for CommandHint {
    fn display(&self) -> &str {
        &self.display
    }

    fn completion(&self) -> Option<&str> {
        if self.complete_up_to > 0 {
            Some(&self.display[..self.complete_up_to])
        } else {
            None
        }
    }
}

impl CommandHint {
    fn new(display: impl Into<String>, complete_up_to: usize) -> Self {
        Self {
            display: display.into(),
            complete_up_to,
        }
    }

    #[allow(dead_code)] // May be useful for future hint refinements
    fn suffix(&self, strip_chars: usize) -> Self {
        Self {
            display: self.display[strip_chars..].to_owned(),
            complete_up_to: self.complete_up_to.saturating_sub(strip_chars),
        }
    }
}

/// RAPS interactive shell helper
pub struct RapsHelper {
    /// Command tree for completion
    commands: Vec<CommandInfo>,
    /// Flat map for quick lookup: "auth login" -> CommandInfo
    command_map: HashMap<String, CommandInfo>,
}

impl RapsHelper {
    pub fn new() -> Self {
        let commands = Self::build_command_tree();
        let command_map = Self::build_command_map(&commands);
        Self {
            commands,
            command_map,
        }
    }

    /// Build the command tree based on RAPS CLI structure
    fn build_command_tree() -> Vec<CommandInfo> {
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
                        name: "assets",
                        description: "Asset management",
                        params: &[],
                        flags: &[],
                        subcommands: &[],
                    },
                    CommandInfo {
                        name: "submittals",
                        description: "Submittals management",
                        params: &[],
                        flags: &[],
                        subcommands: &[],
                    },
                    CommandInfo {
                        name: "checklists",
                        description: "Checklists management",
                        params: &[],
                        flags: &[],
                        subcommands: &[],
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

    /// Format a completion pair with consistent alignment
    fn format_completion(name: &str, description: &str) -> Pair {
        // Use 14-character width for command names (covers longest: "completions")
        // followed by a separator for clear visual distinction
        Pair {
            display: format!("{:<14}  -- {}", name, description),
            replacement: name.to_string(),
        }
    }

    /// Get completions for the current input
    fn get_completions(&self, line: &str) -> Vec<Pair> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut completions = Vec::new();

        match parts.len() {
            0 => {
                // Empty line - suggest all top-level commands
                for cmd in &self.commands {
                    completions.push(Self::format_completion(cmd.name, cmd.description));
                }
            }
            1 => {
                let partial = parts[0].to_lowercase();
                let trailing_space = line.ends_with(' ');

                if trailing_space {
                    // Command is complete, suggest subcommands
                    if let Some(cmd) = self.commands.iter().find(|c| c.name == partial) {
                        for subcmd in cmd.subcommands {
                            completions.push(Self::format_completion(subcmd.name, subcmd.description));
                        }
                    }
                } else {
                    // Partial command - filter matching commands
                    for cmd in &self.commands {
                        if cmd.name.starts_with(&partial) {
                            completions.push(Self::format_completion(cmd.name, cmd.description));
                        }
                    }
                }
            }
            2 => {
                let cmd_name = parts[0].to_lowercase();
                let partial = parts[1].to_lowercase();
                let trailing_space = line.ends_with(' ');

                if let Some(cmd) = self.commands.iter().find(|c| c.name == cmd_name) {
                    if trailing_space {
                        // Subcommand is complete, suggest parameters/flags
                        if let Some(subcmd) = cmd.subcommands.iter().find(|s| s.name == partial) {
                            for flag in subcmd.flags {
                                // Extract flag name for display
                                let flag_name = flag.split_whitespace().next().unwrap_or(flag);
                                completions.push(Pair {
                                    display: format!("{:<20}  (optional)", flag_name),
                                    replacement: flag_name.to_string(),
                                });
                            }
                        }
                    } else {
                        // Partial subcommand - filter matching subcommands
                        for subcmd in cmd.subcommands {
                            if subcmd.name.starts_with(&partial) {
                                completions.push(Self::format_completion(subcmd.name, subcmd.description));
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

                if let Some(cmd) = self.command_map.get(&key) {
                    let last = parts.last().unwrap_or(&"");
                    let trailing_space = line.ends_with(' ');

                    if trailing_space || last.starts_with('-') {
                        for flag in cmd.flags {
                            let flag_name = flag.split_whitespace().next().unwrap_or(flag);
                            if trailing_space || flag_name.starts_with(last) {
                                completions.push(Pair {
                                    display: format!("{:<20}  (optional)", flag_name),
                                    replacement: flag_name.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        completions
    }

    /// Generate a hint for the current input
    fn get_hint(&self, line: &str) -> Option<CommandHint> {
        if line.is_empty() {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let trailing_space = line.ends_with(' ');

        match parts.len() {
            1 if !trailing_space => {
                // Partial command - find matching command and show full name
                let partial = parts[0].to_lowercase();
                for cmd in &self.commands {
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

                        return Some(CommandHint::new(hint, suffix.len()));
                    }
                }
            }
            1 if trailing_space => {
                // Complete command - show subcommands or params
                let cmd_name = parts[0].to_lowercase();
                if let Some(cmd) = self.commands.iter().find(|c| c.name == cmd_name) {
                    if !cmd.subcommands.is_empty() {
                        let subcmd_names: Vec<&str> =
                            cmd.subcommands.iter().take(3).map(|s| s.name).collect();
                        let hint = format!("<{}...>", subcmd_names.join("|"));
                        return Some(CommandHint::new(hint, 0));
                    } else if !cmd.params.is_empty() {
                        let hint = cmd.params.join(" ");
                        return Some(CommandHint::new(hint, 0));
                    }
                }
            }
            2 if !trailing_space => {
                // Partial subcommand
                let cmd_name = parts[0].to_lowercase();
                let partial = parts[1].to_lowercase();

                if let Some(cmd) = self.commands.iter().find(|c| c.name == cmd_name) {
                    for subcmd in cmd.subcommands {
                        if subcmd.name.starts_with(&partial) && subcmd.name != partial {
                            let suffix = &subcmd.name[partial.len()..];
                            let mut hint = suffix.to_string();

                            if !subcmd.params.is_empty() {
                                hint.push(' ');
                                hint.push_str(&subcmd.params.join(" "));
                            }

                            return Some(CommandHint::new(hint, suffix.len()));
                        }
                    }
                }
            }
            2 if trailing_space => {
                // Complete subcommand - show params
                let cmd_name = parts[0].to_lowercase();
                let sub_name = parts[1].to_lowercase();
                let key = format!("{} {}", cmd_name, sub_name);

                if let Some(cmd) = self.command_map.get(&key) {
                    if !cmd.params.is_empty() {
                        let hint = cmd.params.join(" ");
                        return Some(CommandHint::new(hint, 0));
                    } else if !cmd.flags.is_empty() {
                        let hint = format!("[{}]", cmd.flags.first().unwrap_or(&""));
                        return Some(CommandHint::new(hint, 0));
                    }
                }
            }
            n if n >= 3 => {
                // Show remaining params
                let cmd_name = parts[0].to_lowercase();
                let sub_name = parts[1].to_lowercase();
                let key = format!("{} {}", cmd_name, sub_name);

                if let Some(cmd) = self.command_map.get(&key) {
                    // Count how many positional args we have (excluding flags)
                    let positional_count =
                        parts[2..].iter().filter(|p| !p.starts_with('-')).count();

                    if positional_count < cmd.params.len() {
                        let remaining: Vec<&str> =
                            cmd.params.iter().skip(positional_count).copied().collect();
                        if !remaining.is_empty() && trailing_space {
                            let hint = remaining.join(" ");
                            return Some(CommandHint::new(hint, 0));
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }
}

impl Default for RapsHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for RapsHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let completions = self.get_completions(&line[..pos]);

        // Find the start position for replacement
        let start = if line[..pos].ends_with(' ') {
            pos
        } else {
            line[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0)
        };

        Ok((start, completions))
    }
}

impl Hinter for RapsHelper {
    type Hint = CommandHint;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<CommandHint> {
        // Only show hints when cursor is at the end
        if pos < line.len() {
            return None;
        }

        self.get_hint(line)
    }
}

impl Highlighter for RapsHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // Use dim + italic for better visibility across terminals
        // \x1b[2m = dim, \x1b[3m = italic, \x1b[36m = cyan (for dark terminals)
        // Fall back to just dim for maximum compatibility
        Owned(format!("\x1b[2;36m{}\x1b[0m", hint))
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        // Could add syntax highlighting here in the future
        Borrowed(line)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        false
    }
}

impl Validator for RapsHelper {}

impl Helper for RapsHelper {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helper_creation() {
        let helper = RapsHelper::new();
        assert!(!helper.commands.is_empty());
        assert!(!helper.command_map.is_empty());
    }

    #[test]
    fn test_command_completions() {
        let helper = RapsHelper::new();

        // Test empty line
        let completions = helper.get_completions("");
        assert!(!completions.is_empty());

        // Test partial command
        let completions = helper.get_completions("au");
        assert!(completions.iter().any(|c| c.replacement == "auth"));

        // Test command with space
        let completions = helper.get_completions("auth ");
        assert!(completions.iter().any(|c| c.replacement == "login"));

        // Test partial subcommand
        let completions = helper.get_completions("auth log");
        assert!(completions.iter().any(|c| c.replacement == "login"));
        assert!(completions.iter().any(|c| c.replacement == "logout"));
    }

    #[test]
    fn test_hints() {
        let helper = RapsHelper::new();

        // Test partial command hint
        let hint = helper.get_hint("au");
        assert!(hint.is_some());
        assert!(hint.unwrap().display.starts_with("th"));

        // Test complete command shows subcommand hint
        let hint = helper.get_hint("auth ");
        assert!(hint.is_some());

        // Test subcommand with params
        let hint = helper.get_hint("bucket create ");
        assert!(hint.is_some());
        assert!(hint.unwrap().display.contains("BUCKET_KEY"));
    }
}
