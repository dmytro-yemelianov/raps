// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Design Automation commands
//!
//! Commands for managing engines, app bundles, activities, and work items.

mod activities;
mod appbundles;
mod engines;
mod workitems;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use serde::Deserialize;

use crate::output::OutputFormat;
use raps_da::DesignAutomationClient;

#[derive(Debug, Subcommand)]
pub enum DaCommands {
    /// List available engines
    Engines,

    /// List app bundles
    Appbundles,

    /// Create an app bundle
    #[command(name = "appbundle-create")]
    AppbundleCreate {
        /// App bundle ID
        #[arg(short, long)]
        id: Option<String>,

        /// Engine ID (e.g., Autodesk.AutoCAD+24)
        #[arg(short, long)]
        engine: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete an app bundle
    #[command(name = "appbundle-delete")]
    AppbundleDelete {
        /// App bundle ID to delete
        id: String,
    },

    /// Upload a .zip archive for an app bundle
    #[command(name = "appbundle-upload")]
    AppbundleUpload {
        /// App bundle ID (must already exist -- use appbundle-create first)
        id: String,

        /// Path to the .zip archive to upload
        #[arg(short, long)]
        file: PathBuf,

        /// Engine ID (e.g., Autodesk.AutoCAD+24)
        #[arg(short, long)]
        engine: String,

        /// Description for the new version
        #[arg(short, long)]
        description: Option<String>,
    },

    /// List activities
    Activities,

    /// Create an activity from JSON or YAML definition
    #[command(name = "activity-create")]
    ActivityCreate {
        /// Path to JSON or YAML activity definition file (use `-` for stdin, parsed as YAML)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Activity ID (required if not using --file)
        #[arg(long)]
        id: Option<String>,

        /// Engine ID (required if not using --file)
        #[arg(long)]
        engine: Option<String>,

        /// App bundle ID to use (required if not using --file)
        #[arg(long)]
        appbundle: Option<String>,

        /// Command line (required if not using --file)
        #[arg(long)]
        command: Option<String>,

        /// Description
        #[arg(long)]
        description: Option<String>,
    },

    /// Delete an activity
    #[command(name = "activity-delete")]
    ActivityDelete {
        /// Activity ID to delete
        id: String,
    },

    /// Submit a work item to run an activity
    #[command(name = "run")]
    Run {
        /// Activity ID (fully qualified, e.g., owner.activity+alias)
        activity: String,

        /// Input arguments as key=value pairs (use @file.dwg for file inputs)
        #[arg(short, long, value_parser = parse_argument)]
        input: Vec<(String, String)>,

        /// Output arguments as key=value pairs (local file paths)
        #[arg(long = "out-arg", value_parser = parse_argument)]
        out_arg: Vec<(String, String)>,

        /// Wait for completion and download results
        #[arg(short, long)]
        wait: bool,
    },

    /// List recent workitems
    Workitems,

    /// Check work item status
    Status {
        /// Work item ID
        workitem_id: String,

        /// Wait for completion
        #[arg(short, long)]
        wait: bool,

        /// Download outputs on completion
        #[arg(short, long)]
        download: bool,

        /// Output directory for downloads
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}

/// Parse key=value argument pairs
fn parse_argument(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid argument format '{}'. Use key=value", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Activity definition structure for JSON/YAML parsing
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ActivityDefinition {
    pub(super) id: String,
    pub(super) engine: String,
    #[serde(default)]
    pub(super) command_line: Vec<String>,
    #[serde(default)]
    pub(super) app_bundles: Vec<String>,
    #[serde(default)]
    pub(super) parameters: HashMap<String, ParameterDefinition>,
    pub(super) description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ParameterDefinition {
    pub(super) verb: String,
    pub(super) local_name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) required: Option<bool>,
    pub(super) zip: Option<bool>,
}

impl DaCommands {
    pub async fn execute(
        self,
        client: &DesignAutomationClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            DaCommands::Engines => engines::list_engines(client, output_format).await,
            DaCommands::Appbundles => appbundles::list_appbundles(client, output_format).await,
            DaCommands::AppbundleCreate {
                id,
                engine,
                description,
            } => appbundles::create_appbundle(client, id, engine, description, output_format).await,
            DaCommands::AppbundleDelete { id } => {
                appbundles::delete_appbundle(client, &id, output_format).await
            }
            DaCommands::AppbundleUpload {
                id,
                file,
                engine,
                description,
            } => {
                appbundles::upload_appbundle(
                    client,
                    &id,
                    &file,
                    &engine,
                    description,
                    output_format,
                )
                .await
            }
            DaCommands::Activities => activities::list_activities(client, output_format).await,
            DaCommands::ActivityCreate {
                file,
                id,
                engine,
                appbundle,
                command,
                description,
            } => {
                activities::create_activity(
                    client,
                    file,
                    id,
                    engine,
                    appbundle,
                    command,
                    description,
                    output_format,
                )
                .await
            }
            DaCommands::ActivityDelete { id } => {
                activities::delete_activity(client, &id, output_format).await
            }
            DaCommands::Workitems => workitems::list_workitems(client, output_format).await,
            DaCommands::Run {
                activity,
                input,
                out_arg,
                wait,
            } => {
                workitems::run_workitem(client, &activity, input, out_arg, wait, output_format)
                    .await
            }
            DaCommands::Status {
                workitem_id,
                wait,
                download,
                output_dir,
            } => {
                workitems::check_status(
                    client,
                    &workitem_id,
                    wait,
                    download,
                    output_dir,
                    output_format,
                )
                .await
            }
        }
    }
}
