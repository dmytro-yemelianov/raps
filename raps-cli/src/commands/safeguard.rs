// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Rollback and backup script generators.
//!
//! Generates executable shell scripts that capture the current state (backup)
//! or undo an operation (rollback) for any RAPS command.
//!
//! # Usage
//!
//! ```bash
//! # Generate a rollback script for a destructive operation
//! raps safeguard rollback "bucket delete my-bucket"
//!
//! # Generate a backup script to capture current state before changes
//! raps safeguard backup "admin user remove --project-id abc --email user@co.com"
//!
//! # Preview what would be generated (dry-run)
//! raps safeguard rollback "object delete my-bucket/model.rvt" --dry-run
//!
//! # Save to a specific file instead of auto-naming
//! raps safeguard backup "webhook delete hook-id-123" -o my-backup.sh
//! ```

use std::path::PathBuf;

use anyhow::{bail, Result};
use chrono::Utc;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

// ── Clap structs ────────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum SafeguardCommands {
    /// Generate a rollback script that undoes a given raps command
    Rollback {
        /// The raps command to generate rollback for (without "raps" prefix)
        ///
        /// Example: "bucket delete my-bucket"
        command: String,

        /// Output file (default: rollback-<command>-<timestamp>.sh)
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,

        /// Print the script to stdout without writing a file
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate a backup script that captures current state before an operation
    Backup {
        /// The raps command to generate backup for (without "raps" prefix)
        ///
        /// Example: "admin user remove --project-id abc --email user@co.com"
        command: String,

        /// Output file (default: backup-<command>-<timestamp>.sh)
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,

        /// Print the script to stdout without writing a file
        #[arg(long)]
        dry_run: bool,
    },

    /// List all known reversible operations and their inverse commands
    List,
}

impl SafeguardCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            SafeguardCommands::Rollback {
                command,
                out_file,
                dry_run,
            } => generate_rollback(&command, out_file, dry_run, output_format),
            SafeguardCommands::Backup {
                command,
                out_file,
                dry_run,
            } => generate_backup(&command, out_file, dry_run, output_format),
            SafeguardCommands::List => list_operations(output_format),
        }
    }
}

// ── Operation registry ──────────────────────────────────────────────────────

/// Describes how to reverse or back up an operation.
struct OperationDef {
    /// Pattern to match: "bucket delete", "object upload", etc.
    pattern: &'static str,
    /// Human description
    description: &'static str,
    /// Generate rollback script lines from parsed args
    rollback: fn(&ParsedCommand) -> Option<Vec<ScriptLine>>,
    /// Generate backup script lines from parsed args
    backup: fn(&ParsedCommand) -> Option<Vec<ScriptLine>>,
}

#[derive(Debug)]
struct ParsedCommand {
    /// The full original command string
    #[allow(dead_code)]
    raw: String,
    /// Positional args (after the subcommand words)
    positionals: Vec<String>,
    /// Named flags: --key value
    flags: std::collections::HashMap<String, String>,
    /// Boolean flags: --yes, --force
    bool_flags: Vec<String>,
}

impl ParsedCommand {
    fn flag(&self, key: &str) -> Option<&str> {
        self.flags.get(key).map(|s| s.as_str())
    }

    fn pos(&self, idx: usize) -> Option<&str> {
        self.positionals.get(idx).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
struct ScriptLine {
    comment: Option<String>,
    command: Option<String>,
}

impl ScriptLine {
    fn comment(s: &str) -> Self {
        Self {
            comment: Some(s.to_string()),
            command: None,
        }
    }
    fn cmd(s: &str) -> Self {
        Self {
            comment: None,
            command: Some(s.to_string()),
        }
    }
    fn both(comment: &str, cmd: &str) -> Self {
        Self {
            comment: Some(comment.to_string()),
            command: Some(cmd.to_string()),
        }
    }
}

/// All known reversible operations.
fn operation_registry() -> Vec<OperationDef> {
    vec![
        // ── Bucket operations ───────────────────────────────────────────
        OperationDef {
            pattern: "bucket create",
            description: "Create an OSS bucket",
            rollback: |cmd| {
                let bucket = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::both("Delete the bucket that was created", &format!("raps bucket delete {bucket} --yes")),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state to back up — bucket did not exist")])
            },
        },
        OperationDef {
            pattern: "bucket delete",
            description: "Delete an OSS bucket",
            rollback: |cmd| {
                let bucket = cmd.pos(0)?;
                let policy = cmd.flag("policy").unwrap_or("transient");
                Some(vec![
                    ScriptLine::both("Re-create the deleted bucket", &format!("raps bucket create {bucket} --policy {policy}")),
                    ScriptLine::comment("NOTE: Objects that were in the bucket cannot be automatically restored."),
                    ScriptLine::comment("If a backup script was generated beforehand, run it to re-upload objects."),
                ])
            },
            backup: |cmd| {
                let bucket = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::both("Save bucket metadata", &format!("raps bucket info {bucket} --output json > \"backup-bucket-{bucket}-info.json\"")),
                    ScriptLine::both("Take a snapshot of all objects", &format!("raps snapshot create {bucket}")),
                    ScriptLine::both("Download all objects locally", &format!("mkdir -p \"backup-{bucket}\" && raps object list {bucket} --output json | jq -r '.[].objectKey' | while read -r key; do raps object download \"{bucket}/$key\" \"backup-{bucket}/$key\"; done")),
                ])
            },
        },
        // ── Object operations ───────────────────────────────────────────
        OperationDef {
            pattern: "object upload",
            description: "Upload object(s) to a bucket",
            rollback: |cmd| {
                // raps object upload <bucket>/<key> <file> OR raps object upload-batch ...
                let target = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::both("Delete the uploaded object", &format!("raps object delete {target} --yes")),
                ])
            },
            backup: |cmd| {
                let target = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::comment("Check if object existed before upload (would be overwritten)"),
                    ScriptLine::both("Download existing version if present", &format!(
                        "raps object info {target} --output json > /dev/null 2>&1 && raps object download {target} \"backup-$(echo {target} | tr '/' '-')\""
                    )),
                ])
            },
        },
        OperationDef {
            pattern: "object delete",
            description: "Delete object(s) from a bucket",
            rollback: |cmd| {
                let target = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::comment("Cannot automatically restore deleted objects."),
                    ScriptLine::comment("If a backup was made, re-upload from the backup file:"),
                    ScriptLine::cmd(&format!("# raps object upload {target} \"backup-$(echo {target} | tr '/' '-')\""))
                ])
            },
            backup: |cmd| {
                let target = cmd.pos(0)?;
                let safe_name = target.replace('/', "-");
                Some(vec![
                    ScriptLine::both("Download object before deletion", &format!("raps object download {target} \"backup-{safe_name}\"")),
                ])
            },
        },
        OperationDef {
            pattern: "object copy",
            description: "Copy object between buckets",
            rollback: |cmd| {
                let target = cmd.flag("to")?;
                Some(vec![
                    ScriptLine::both("Delete the copied object at destination", &format!("raps object delete {target} --yes")),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("Copy is non-destructive — no backup needed")])
            },
        },
        // ── Translate operations ────────────────────────────────────────
        OperationDef {
            pattern: "translate start",
            description: "Start a model translation job",
            rollback: |cmd| {
                let urn = cmd.pos(0).or_else(|| cmd.flag("urn"))?;
                Some(vec![
                    ScriptLine::comment("Translation jobs cannot be cancelled once started."),
                    ScriptLine::comment("To clean up the translated output:"),
                    ScriptLine::cmd(&format!("# raps api delete /modelderivative/v2/designdata/{urn}/manifest")),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("Translation is additive — no prior state to back up")])
            },
        },
        // ── Webhook operations ──────────────────────────────────────────
        OperationDef {
            pattern: "webhook create",
            description: "Create a webhook subscription",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Delete the webhook that was created."),
                    ScriptLine::comment("Note: Webhook ID is returned by the create command."),
                    ScriptLine::cmd("# raps webhook delete <WEBHOOK_ID>"),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state — webhook did not exist")])
            },
        },
        OperationDef {
            pattern: "webhook delete",
            description: "Delete a webhook subscription",
            rollback: |cmd| {
                let hook_id = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::comment("Cannot automatically re-create a deleted webhook."),
                    ScriptLine::comment("If a backup was made, re-create from the saved config:"),
                    ScriptLine::cmd(&format!("# Review backup-webhook-{hook_id}.json and re-create manually")),
                    ScriptLine::cmd("# raps webhook create --system <system> --event <event> --callback-url <url>"),
                ])
            },
            backup: |cmd| {
                let hook_id = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::both("Save webhook config before deletion", &format!("raps webhook get {hook_id} --output json > \"backup-webhook-{hook_id}.json\"")),
                ])
            },
        },
        OperationDef {
            pattern: "webhook update",
            description: "Update a webhook subscription",
            rollback: |cmd| {
                let hook_id = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::comment("Restore webhook to previous state from backup:"),
                    ScriptLine::cmd(&format!("# Review backup-webhook-{hook_id}.json and apply previous values")),
                    ScriptLine::cmd(&format!("# raps webhook update {hook_id} --status <old-status>")),
                ])
            },
            backup: |cmd| {
                let hook_id = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::both("Save current webhook config", &format!("raps webhook get {hook_id} --output json > \"backup-webhook-{hook_id}.json\"")),
                ])
            },
        },
        // ── Project operations ──────────────────────────────────────────
        OperationDef {
            pattern: "project create",
            description: "Create a new project",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Projects cannot be deleted, only archived."),
                    ScriptLine::comment("Archive the created project:"),
                    ScriptLine::cmd("# raps project archive <PROJECT_ID>"),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state — project did not exist")])
            },
        },
        OperationDef {
            pattern: "project update",
            description: "Update project settings",
            rollback: |cmd| {
                let project_id = cmd.pos(0).or_else(|| cmd.flag("project-id"))?;
                Some(vec![
                    ScriptLine::comment("Restore project to previous state from backup:"),
                    ScriptLine::cmd(&format!("# Review backup-project-{project_id}.json and apply previous values")),
                    ScriptLine::cmd(&format!("# raps project update {project_id} --name <old-name> ...")),
                ])
            },
            backup: |cmd| {
                let project_id = cmd.pos(0).or_else(|| cmd.flag("project-id"))?;
                Some(vec![
                    ScriptLine::both("Save current project state", &format!("raps project info {project_id} --output json > \"backup-project-{project_id}.json\"")),
                ])
            },
        },
        OperationDef {
            pattern: "project archive",
            description: "Archive a project",
            rollback: |cmd| {
                let project_id = cmd.pos(0).or_else(|| cmd.flag("project-id"))?;
                Some(vec![
                    ScriptLine::both("Restore archived project", &format!("raps project update {project_id} --status active")),
                ])
            },
            backup: |cmd| {
                let project_id = cmd.pos(0).or_else(|| cmd.flag("project-id"))?;
                Some(vec![
                    ScriptLine::both("Save project state before archiving", &format!("raps project info {project_id} --output json > \"backup-project-{project_id}.json\"")),
                ])
            },
        },
        // ── Admin user operations ───────────────────────────────────────
        OperationDef {
            pattern: "admin user add",
            description: "Add user(s) to project(s)",
            rollback: |cmd| {
                let email = cmd.flag("email")?;
                let project_id = cmd.flag("project-id")?;
                Some(vec![
                    ScriptLine::both("Remove the user that was added", &format!("raps admin user remove --project-id {project_id} --email {email}")),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state — user was not in the project")])
            },
        },
        OperationDef {
            pattern: "admin user remove",
            description: "Remove user(s) from project(s)",
            rollback: |cmd| {
                let email = cmd.flag("email")?;
                let project_id = cmd.flag("project-id")?;
                let role = cmd.flag("role").unwrap_or("Project Member");
                Some(vec![
                    ScriptLine::comment("Re-add the removed user with their previous role:"),
                    ScriptLine::cmd(&format!("raps admin user add --project-id {project_id} --email {email} --role \"{role}\"")),
                    ScriptLine::comment("NOTE: If the backup captured a different role, use the role from the backup file."),
                ])
            },
            backup: |cmd| {
                let email = cmd.flag("email");
                let project_id = cmd.flag("project-id")?;
                let safe = project_id.replace(':', "-");
                Some(vec![
                    ScriptLine::both("Save current project users list",
                        &format!("raps admin user list --project-id {project_id} --output json > \"backup-users-{safe}.json\"")),
                    if let Some(e) = email {
                        ScriptLine::comment(&format!("Specifically backing up user: {e}"))
                    } else {
                        ScriptLine::comment("Backing up all users in project")
                    },
                ])
            },
        },
        OperationDef {
            pattern: "admin user update",
            description: "Update user role in project",
            rollback: |cmd| {
                let email = cmd.flag("email")?;
                let project_id = cmd.flag("project-id")?;
                Some(vec![
                    ScriptLine::comment("Restore user to their previous role from backup:"),
                    ScriptLine::cmd(&format!("# Review backup and run:")),
                    ScriptLine::cmd(&format!("# raps admin user update --project-id {project_id} --email {email} --role \"<PREVIOUS_ROLE>\"")),
                ])
            },
            backup: |cmd| {
                let email = cmd.flag("email");
                let project_id = cmd.flag("project-id")?;
                let safe = project_id.replace(':', "-");
                Some(vec![
                    ScriptLine::both("Save current user roles",
                        &format!("raps admin user list --project-id {project_id} --output json > \"backup-users-{safe}.json\"")),
                    if let Some(e) = email {
                        ScriptLine::comment(&format!("Tracking role for: {e}"))
                    } else {
                        ScriptLine::comment("All users saved")
                    },
                ])
            },
        },
        // ── Admin CSV operations ────────────────────────────────────────
        OperationDef {
            pattern: "admin user import",
            description: "Bulk import users from CSV",
            rollback: |cmd| {
                let csv = cmd.pos(0).or_else(|| cmd.flag("csv"))?;
                Some(vec![
                    ScriptLine::comment("To undo a bulk import, remove all users added by the CSV."),
                    ScriptLine::comment("Generate a removal script from the same CSV:"),
                    ScriptLine::cmd(&format!("# Parse {csv} and run 'raps admin user remove' for each row")),
                    ScriptLine::cmd(&format!("awk -F',' 'NR>1 {{print \"raps admin user remove --project-id \" $3 \" --email \" $1}}' {csv} > rollback-import.sh")),
                    ScriptLine::cmd("chmod +x rollback-import.sh"),
                ])
            },
            backup: |cmd| {
                let project_id = cmd.flag("project-id");
                match project_id {
                    Some(pid) => {
                        let safe = pid.replace(':', "-");
                        Some(vec![
                            ScriptLine::both("Save current users before import",
                                &format!("raps admin user list --project-id {pid} --output json > \"backup-pre-import-{safe}.json\"")),
                        ])
                    }
                    None => Some(vec![
                        ScriptLine::comment("Bulk import may affect multiple projects."),
                        ScriptLine::comment("Use 'raps admin user list' per project to capture state."),
                    ]),
                }
            },
        },
        // ── Admin folder permissions ────────────────────────────────────
        OperationDef {
            pattern: "admin folder set-permissions",
            description: "Set folder-level permission overrides",
            rollback: |cmd| {
                let project_id = cmd.flag("project-id")?;
                let folder_id = cmd.flag("folder-id").or_else(|| cmd.pos(0))?;
                Some(vec![
                    ScriptLine::comment("Restore previous folder permissions from backup:"),
                    ScriptLine::cmd(&format!("# Review backup-folder-perms-{folder_id}.json")),
                    ScriptLine::cmd(&format!("# raps admin folder set-permissions --project-id {project_id} --folder-id {folder_id} ...")),
                ])
            },
            backup: |cmd| {
                let project_id = cmd.flag("project-id")?;
                let folder_id = cmd.flag("folder-id").or_else(|| cmd.pos(0)).unwrap_or("all");
                let safe = folder_id.replace(':', "-");
                Some(vec![
                    ScriptLine::both("Save current folder permissions",
                        &format!("raps folder list --project-id {project_id} --output json > \"backup-folder-perms-{safe}.json\"")),
                ])
            },
        },
        // ── Issue operations ────────────────────────────────────────────
        OperationDef {
            pattern: "issue create",
            description: "Create an issue",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Issues cannot be deleted via API, only closed/voided:"),
                    ScriptLine::cmd("# raps issue update <ISSUE_ID> --status void"),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state — issue did not exist")])
            },
        },
        OperationDef {
            pattern: "issue update",
            description: "Update an issue",
            rollback: |cmd| {
                let issue_id = cmd.pos(0).or_else(|| cmd.flag("issue-id"))?;
                Some(vec![
                    ScriptLine::comment("Restore issue to previous state from backup:"),
                    ScriptLine::cmd(&format!("# Review backup-issue-{issue_id}.json")),
                    ScriptLine::cmd(&format!("# raps issue update {issue_id} --status <old-status> --title <old-title> ...")),
                ])
            },
            backup: |cmd| {
                let issue_id = cmd.pos(0).or_else(|| cmd.flag("issue-id"))?;
                let project_id = cmd.flag("project-id").unwrap_or("unknown");
                Some(vec![
                    ScriptLine::both("Save current issue state", &format!(
                        "raps issue get {issue_id} --project-id {project_id} --output json > \"backup-issue-{issue_id}.json\""
                    )),
                ])
            },
        },
        // ── RFI operations ──────────────────────────────────────────────
        OperationDef {
            pattern: "rfi create",
            description: "Create an RFI",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("RFIs cannot be deleted, only closed:"),
                    ScriptLine::cmd("# raps rfi update <RFI_ID> --status closed"),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state — RFI did not exist")])
            },
        },
        OperationDef {
            pattern: "rfi update",
            description: "Update an RFI",
            rollback: |cmd| {
                let rfi_id = cmd.pos(0).or_else(|| cmd.flag("rfi-id"))?;
                Some(vec![
                    ScriptLine::comment("Restore RFI to previous state from backup:"),
                    ScriptLine::cmd(&format!("# Review backup-rfi-{rfi_id}.json and apply old values")),
                ])
            },
            backup: |cmd| {
                let rfi_id = cmd.pos(0).or_else(|| cmd.flag("rfi-id"))?;
                let project_id = cmd.flag("project-id").unwrap_or("unknown");
                Some(vec![
                    ScriptLine::both("Save current RFI state", &format!(
                        "raps rfi get {rfi_id} --project-id {project_id} --output json > \"backup-rfi-{rfi_id}.json\""
                    )),
                ])
            },
        },
        // ── Template operations ─────────────────────────────────────────
        OperationDef {
            pattern: "template create",
            description: "Create a project template",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Archive the created template:"),
                    ScriptLine::cmd("# raps template archive <TEMPLATE_ID>"),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state — template did not exist")])
            },
        },
        OperationDef {
            pattern: "template update",
            description: "Update a project template",
            rollback: |cmd| {
                let tmpl_id = cmd.pos(0).or_else(|| cmd.flag("template-id"))?;
                Some(vec![
                    ScriptLine::comment("Restore template from backup:"),
                    ScriptLine::cmd(&format!("# Review backup-template-{tmpl_id}.json")),
                ])
            },
            backup: |cmd| {
                let tmpl_id = cmd.pos(0).or_else(|| cmd.flag("template-id"))?;
                Some(vec![
                    ScriptLine::both("Save current template state", &format!(
                        "raps template info {tmpl_id} --output json > \"backup-template-{tmpl_id}.json\""
                    )),
                ])
            },
        },
        OperationDef {
            pattern: "template archive",
            description: "Archive a project template",
            rollback: |cmd| {
                let tmpl_id = cmd.pos(0).or_else(|| cmd.flag("template-id"))?;
                Some(vec![
                    ScriptLine::both("Restore archived template", &format!("raps template update {tmpl_id} --status active")),
                ])
            },
            backup: |cmd| {
                let tmpl_id = cmd.pos(0).or_else(|| cmd.flag("template-id"))?;
                Some(vec![
                    ScriptLine::both("Save template state", &format!(
                        "raps template info {tmpl_id} --output json > \"backup-template-{tmpl_id}.json\""
                    )),
                ])
            },
        },
        // ── Folder operations ───────────────────────────────────────────
        OperationDef {
            pattern: "folder create",
            description: "Create a folder in a project",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Folders cannot be deleted via APS API."),
                    ScriptLine::comment("The created folder will remain in the project."),
                ])
            },
            backup: |_cmd| {
                Some(vec![ScriptLine::comment("No prior state — folder did not exist")])
            },
        },
        // ── Item operations ─────────────────────────────────────────────
        OperationDef {
            pattern: "item delete",
            description: "Delete a file/item from a project",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Deleted items may be recoverable from the project's recycle bin."),
                    ScriptLine::comment("If a backup was made, re-upload the file to restore it."),
                ])
            },
            backup: |cmd| {
                let item_id = cmd.pos(0).or_else(|| cmd.flag("item-id"))?;
                Some(vec![
                    ScriptLine::both("Save item metadata", &format!(
                        "raps item info {item_id} --output json > \"backup-item-{item_id}.json\""
                    )),
                    ScriptLine::comment("Consider downloading the file content if needed:"),
                    ScriptLine::cmd(&format!("# raps item download {item_id} \"backup-item-{item_id}-content\"")),
                ])
            },
        },
        OperationDef {
            pattern: "item rename",
            description: "Rename a file/item",
            rollback: |cmd| {
                let item_id = cmd.pos(0).or_else(|| cmd.flag("item-id"))?;
                Some(vec![
                    ScriptLine::comment("Rename back to previous name:"),
                    ScriptLine::cmd(&format!("# Review backup-item-{item_id}.json for original name")),
                    ScriptLine::cmd(&format!("# raps item rename {item_id} --name \"<ORIGINAL_NAME>\"")),
                ])
            },
            backup: |cmd| {
                let item_id = cmd.pos(0).or_else(|| cmd.flag("item-id"))?;
                Some(vec![
                    ScriptLine::both("Save current item metadata", &format!(
                        "raps item info {item_id} --output json > \"backup-item-{item_id}.json\""
                    )),
                ])
            },
        },
        // ── Design Automation ───────────────────────────────────────────
        OperationDef {
            pattern: "da workitem create",
            description: "Submit a Design Automation workitem",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Workitems cannot be undone — they may have already modified output files."),
                    ScriptLine::comment("Check the workitem status and review output artifacts."),
                    ScriptLine::cmd("# raps da workitem status <WORKITEM_ID>"),
                ])
            },
            backup: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Consider backing up input/output files before running DA workitems."),
                    ScriptLine::comment("Use 'raps object download' to save input files."),
                ])
            },
        },
        // ── Reality Capture ─────────────────────────────────────────────
        OperationDef {
            pattern: "reality delete",
            description: "Delete a reality capture photoscene",
            rollback: |_cmd| {
                Some(vec![
                    ScriptLine::comment("Deleted photoscenes cannot be restored."),
                    ScriptLine::comment("Re-create from source images using 'raps reality create'."),
                ])
            },
            backup: |cmd| {
                let scene_id = cmd.pos(0).or_else(|| cmd.flag("photoscene-id"))?;
                Some(vec![
                    ScriptLine::both("Save photoscene metadata", &format!(
                        "raps reality status {scene_id} --output json > \"backup-reality-{scene_id}.json\""
                    )),
                    ScriptLine::both("Download result if available", &format!(
                        "raps reality result {scene_id} --output json > \"backup-reality-result-{scene_id}.json\""
                    )),
                ])
            },
        },
        // ── Config operations ───────────────────────────────────────────
        OperationDef {
            pattern: "config set",
            description: "Set a configuration value",
            rollback: |cmd| {
                let key = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::comment("Restore previous config value:"),
                    ScriptLine::cmd(&format!("# raps config set {key} \"<PREVIOUS_VALUE>\"")),
                    ScriptLine::comment("Or remove the override entirely:"),
                    ScriptLine::cmd(&format!("# raps config unset {key}")),
                ])
            },
            backup: |cmd| {
                let key = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::both("Save current config", &format!(
                        "raps config get {key} > \"backup-config-{key}.txt\" 2>/dev/null || echo 'not set' > \"backup-config-{key}.txt\""
                    )),
                ])
            },
        },
        // ── Sync operations ─────────────────────────────────────────────
        OperationDef {
            pattern: "sync",
            description: "Sync a directory to an OSS bucket",
            rollback: |cmd| {
                let bucket = cmd.pos(1).or_else(|| cmd.flag("bucket"))?;
                Some(vec![
                    ScriptLine::comment("Sync may have added/updated/deleted objects in the bucket."),
                    ScriptLine::comment("Restore from a pre-sync snapshot:"),
                    ScriptLine::cmd(&format!("# raps snapshot diff <pre-sync-snapshot>.json <post-sync-snapshot>.json")),
                    ScriptLine::cmd(&format!("# Then manually restore changed objects in {bucket}")),
                ])
            },
            backup: |cmd| {
                let bucket = cmd.pos(1).or_else(|| cmd.flag("bucket"))?;
                Some(vec![
                    ScriptLine::both("Snapshot bucket state before sync", &format!("raps snapshot create {bucket}")),
                ])
            },
        },
        // ── Pipeline operations ─────────────────────────────────────────
        OperationDef {
            pattern: "pipeline run",
            description: "Execute a multi-step pipeline",
            rollback: |cmd| {
                let file = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::comment("Pipelines may execute multiple operations."),
                    ScriptLine::comment("Generate individual rollback scripts for each step:"),
                    ScriptLine::cmd(&format!("# Review pipeline file: {file}")),
                    ScriptLine::cmd("# Run 'raps safeguard rollback' for each mutating step"),
                ])
            },
            backup: |cmd| {
                let file = cmd.pos(0)?;
                Some(vec![
                    ScriptLine::comment(&format!("Pipeline: {file}")),
                    ScriptLine::comment("For comprehensive backup, generate backup scripts per step:"),
                    ScriptLine::cmd("# Run 'raps safeguard backup' for each mutating step in the pipeline"),
                ])
            },
        },
    ]
}

// ── Command parser ──────────────────────────────────────────────────────────

fn parse_command(raw: &str) -> ParsedCommand {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    let mut positionals = Vec::new();
    let mut flags = std::collections::HashMap::new();
    let mut bool_flags = Vec::new();

    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if part.starts_with("--") {
            let key = part.trim_start_matches("--");
            // Check if next part is a value or another flag
            if i + 1 < parts.len() && !parts[i + 1].starts_with("--") {
                flags.insert(key.to_string(), parts[i + 1].to_string());
                i += 2;
            } else {
                bool_flags.push(key.to_string());
                i += 1;
            }
        } else if part.starts_with('-') && part.len() == 2 {
            let key = part.trim_start_matches('-');
            if i + 1 < parts.len() && !parts[i + 1].starts_with('-') {
                flags.insert(key.to_string(), parts[i + 1].to_string());
                i += 2;
            } else {
                bool_flags.push(key.to_string());
                i += 1;
            }
        } else {
            positionals.push(part.to_string());
            i += 1;
        }
    }

    ParsedCommand {
        raw: raw.to_string(),
        positionals,
        flags,
        bool_flags,
    }
}

/// Strip known subcommand prefixes from positionals to get only args.
fn strip_prefix(cmd: &mut ParsedCommand, prefix_words: usize) {
    if cmd.positionals.len() >= prefix_words {
        cmd.positionals = cmd.positionals[prefix_words..].to_vec();
    }
}

// ── Script generation ───────────────────────────────────────────────────────

fn build_script(kind: &str, command: &str, lines: &[ScriptLine]) -> String {
    let now = Utc::now().to_rfc3339();
    let mut script = String::new();

    script.push_str("#!/usr/bin/env bash\n");
    script.push_str(&format!("# RAPS {kind} Script\n"));
    script.push_str(&format!("# Generated: {now}\n"));
    script.push_str(&format!("# Original command: raps {command}\n"));
    script.push_str("#\n");
    script.push_str(&format!(
        "# This script {} the operation above.\n",
        if kind == "Rollback" {
            "undoes"
        } else {
            "captures state before"
        }
    ));
    script.push_str("# Review carefully before executing.\n");
    script.push_str("\nset -euo pipefail\n\n");

    for line in lines {
        if let Some(c) = &line.comment {
            script.push_str(&format!("# {c}\n"));
        }
        if let Some(cmd) = &line.command {
            script.push_str(&format!("{cmd}\n"));
        }
        if line.comment.is_some() || line.command.is_some() {
            script.push('\n');
        }
    }

    script.push_str(&format!("echo \"✓ RAPS {kind} script completed.\"\n"));
    script
}

fn find_operation<'a>(
    registry: &'a [OperationDef],
    command: &str,
) -> Option<&'a OperationDef> {
    let normalized = command.trim().to_lowercase();
    // Try longest prefix match first
    let mut best: Option<&OperationDef> = None;
    let mut best_len = 0;
    for op in registry {
        if normalized.starts_with(op.pattern) && op.pattern.len() > best_len {
            best = Some(op);
            best_len = op.pattern.len();
        }
    }
    best
}

// ── Subcommand implementations ──────────────────────────────────────────────

fn generate_rollback(
    command: &str,
    output: Option<PathBuf>,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let registry = operation_registry();
    let op = find_operation(&registry, command);

    let Some(op) = op else {
        bail!(
            "No rollback strategy known for: raps {command}\n\
             Run 'raps safeguard list' to see supported operations."
        );
    };

    let mut parsed = parse_command(command);
    let prefix_words = op.pattern.split_whitespace().count();
    strip_prefix(&mut parsed, prefix_words);

    let Some(lines) = (op.rollback)(&parsed) else {
        bail!(
            "Could not parse arguments for: raps {command}\n\
             Check that all required arguments are provided."
        );
    };

    let script = build_script("Rollback", command, &lines);

    if dry_run {
        println!("{script}");
        return Ok(());
    }

    let out_file = output.unwrap_or_else(|| {
        let ts = Utc::now().format("%Y%m%dT%H%M%S");
        let slug: String = command
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join("-");
        PathBuf::from(format!("rollback-{slug}-{ts}.sh"))
    });

    std::fs::write(&out_file, &script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&out_file, std::fs::Permissions::from_mode(0o755))?;
    }

    match output_format {
        OutputFormat::Table => {
            println!("{} Rollback script generated", "✓".green().bold());
            println!("  {} raps {}", "Command:".bold(), command);
            println!("  {} {}", "File:".bold(), out_file.display());
            println!(
                "  {} Review the script, then: {}",
                "Next:".bold(),
                format!("bash {}", out_file.display()).cyan()
            );
        }
        _ => {
            #[derive(Serialize, schemars::JsonSchema)]
            struct RollbackOutput {
                file: String,
                command: String,
                operation: String,
            }
            output_format.write(&RollbackOutput {
                file: out_file.display().to_string(),
                command: format!("raps {command}"),
                operation: op.description.to_string(),
            })?;
        }
    }

    Ok(())
}

fn generate_backup(
    command: &str,
    output: Option<PathBuf>,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let registry = operation_registry();
    let op = find_operation(&registry, command);

    let Some(op) = op else {
        bail!(
            "No backup strategy known for: raps {command}\n\
             Run 'raps safeguard list' to see supported operations."
        );
    };

    let mut parsed = parse_command(command);
    let prefix_words = op.pattern.split_whitespace().count();
    strip_prefix(&mut parsed, prefix_words);

    let Some(lines) = (op.backup)(&parsed) else {
        bail!(
            "Could not parse arguments for: raps {command}\n\
             Check that all required arguments are provided."
        );
    };

    let script = build_script("Backup", command, &lines);

    if dry_run {
        println!("{script}");
        return Ok(());
    }

    let out_file = output.unwrap_or_else(|| {
        let ts = Utc::now().format("%Y%m%dT%H%M%S");
        let slug: String = command
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join("-");
        PathBuf::from(format!("backup-{slug}-{ts}.sh"))
    });

    std::fs::write(&out_file, &script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&out_file, std::fs::Permissions::from_mode(0o755))?;
    }

    match output_format {
        OutputFormat::Table => {
            println!("{} Backup script generated", "✓".green().bold());
            println!("  {} raps {}", "Command:".bold(), command);
            println!("  {} {}", "File:".bold(), out_file.display());
            println!(
                "  {} Run {} {}",
                "Next:".bold(),
                format!("bash {}", out_file.display()).cyan(),
                "before the destructive operation".dimmed()
            );
        }
        _ => {
            #[derive(Serialize, schemars::JsonSchema)]
            struct BackupOutput {
                file: String,
                command: String,
                operation: String,
            }
            output_format.write(&BackupOutput {
                file: out_file.display().to_string(),
                command: format!("raps {command}"),
                operation: op.description.to_string(),
            })?;
        }
    }

    Ok(())
}

fn list_operations(output_format: OutputFormat) -> Result<()> {
    let registry = operation_registry();

    match output_format {
        OutputFormat::Table => {
            println!("{}", "Safeguard Operations".bold());
            println!(
                "{}",
                "Generate rollback or backup scripts for these operations:".dimmed()
            );
            println!("{}", "─".repeat(70));
            println!(
                "  {:<32} {}",
                "Command".bold(),
                "Description".bold()
            );
            println!("{}", "─".repeat(70));
            for op in &registry {
                println!(
                    "  {:<32} {}",
                    format!("raps {}", op.pattern).cyan(),
                    op.description
                );
            }
            println!("{}", "─".repeat(70));
            println!(
                "\n  {} raps safeguard rollback \"<command>\"",
                "Usage:".bold()
            );
            println!(
                "         raps safeguard backup  \"<command>\"\n"
            );
        }
        _ => {
            #[derive(Serialize, schemars::JsonSchema)]
            struct OpInfo {
                command: String,
                description: String,
            }
            let ops: Vec<OpInfo> = registry
                .iter()
                .map(|o| OpInfo {
                    command: format!("raps {}", o.pattern),
                    description: o.description.to_string(),
                })
                .collect();
            output_format.write(&ops)?;
        }
    }

    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let cmd = parse_command("bucket delete my-bucket --yes");
        assert_eq!(cmd.positionals, vec!["bucket", "delete", "my-bucket"]);
        assert!(cmd.bool_flags.contains(&"yes".to_string()));
    }

    #[test]
    fn test_parse_flags() {
        let cmd = parse_command("admin user add --email test@co.com --project-id abc123 --role Admin");
        assert_eq!(cmd.flag("email"), Some("test@co.com"));
        assert_eq!(cmd.flag("project-id"), Some("abc123"));
        assert_eq!(cmd.flag("role"), Some("Admin"));
    }

    #[test]
    fn test_find_operation_exact() {
        let registry = operation_registry();
        let op = find_operation(&registry, "bucket delete my-bucket");
        assert!(op.is_some());
        assert_eq!(op.unwrap().pattern, "bucket delete");
    }

    #[test]
    fn test_find_operation_longest_match() {
        let registry = operation_registry();
        let op = find_operation(&registry, "admin user add --email a@b.com --project-id x");
        assert!(op.is_some());
        assert_eq!(op.unwrap().pattern, "admin user add");
    }

    #[test]
    fn test_find_operation_unknown() {
        let registry = operation_registry();
        let op = find_operation(&registry, "unknown command");
        assert!(op.is_none());
    }

    #[test]
    fn test_rollback_bucket_delete() {
        let registry = operation_registry();
        let op = find_operation(&registry, "bucket delete my-bucket").unwrap();
        let mut parsed = parse_command("bucket delete my-bucket");
        strip_prefix(&mut parsed, 2);
        let lines = (op.rollback)(&parsed).unwrap();
        let script = build_script("Rollback", "bucket delete my-bucket", &lines);
        assert!(script.contains("raps bucket create my-bucket"));
    }

    #[test]
    fn test_backup_bucket_delete() {
        let registry = operation_registry();
        let op = find_operation(&registry, "bucket delete my-bucket").unwrap();
        let mut parsed = parse_command("bucket delete my-bucket");
        strip_prefix(&mut parsed, 2);
        let lines = (op.backup)(&parsed).unwrap();
        let script = build_script("Backup", "bucket delete my-bucket", &lines);
        assert!(script.contains("raps snapshot create my-bucket"));
        assert!(script.contains("raps bucket info my-bucket"));
    }

    #[test]
    fn test_rollback_admin_user_add() {
        let registry = operation_registry();
        let op = find_operation(
            &registry,
            "admin user add --email u@x.com --project-id p123",
        )
        .unwrap();
        let mut parsed = parse_command("admin user add --email u@x.com --project-id p123");
        strip_prefix(&mut parsed, 3);
        let lines = (op.rollback)(&parsed).unwrap();
        let script = build_script("Rollback", "admin user add", &lines);
        assert!(script.contains("raps admin user remove"));
        assert!(script.contains("u@x.com"));
        assert!(script.contains("p123"));
    }

    #[test]
    fn test_backup_object_delete() {
        let registry = operation_registry();
        let op = find_operation(&registry, "object delete my-bucket/file.rvt").unwrap();
        let mut parsed = parse_command("object delete my-bucket/file.rvt");
        strip_prefix(&mut parsed, 2);
        let lines = (op.backup)(&parsed).unwrap();
        let script = build_script("Backup", "object delete my-bucket/file.rvt", &lines);
        assert!(script.contains("raps object download my-bucket/file.rvt"));
    }

    #[test]
    fn test_strip_prefix() {
        let mut cmd = parse_command("admin user add --email test@x.com");
        strip_prefix(&mut cmd, 3);
        assert!(cmd.positionals.is_empty());
        assert_eq!(cmd.flag("email"), Some("test@x.com"));
    }

    #[test]
    fn test_script_structure() {
        let lines = vec![
            ScriptLine::comment("Do something"),
            ScriptLine::cmd("raps bucket create test"),
        ];
        let script = build_script("Rollback", "bucket delete test", &lines);
        assert!(script.starts_with("#!/usr/bin/env bash"));
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("RAPS Rollback Script"));
        assert!(script.contains("raps bucket delete test"));
        assert!(script.contains("✓ RAPS Rollback script completed"));
    }
}
