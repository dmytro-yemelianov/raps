// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Translation commands for Model Derivative API
//!
//! Commands for starting translations, checking status, viewing manifests,
//! downloading derivatives, querying metadata, and managing presets.

mod metadata;
mod presets;
mod translations;
mod watch;

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

use crate::output::OutputFormat;
use raps_derivative::DerivativeClient;

#[derive(Debug, Subcommand)]
pub enum TranslateCommands {
    /// Start a translation job
    Start {
        /// Base64-encoded URN of the source file
        urn: Option<String>,

        /// Output format (svf2, svf, obj, stl, step, iges, ifc)
        #[arg(short, long)]
        format: Option<String>,

        /// Root filename (for ZIP files with multiple design files)
        #[arg(short, long)]
        root_filename: Option<String>,

        /// Wait for translation to complete (polls until done)
        #[arg(short, long)]
        wait: bool,

        /// Watch translation until complete, polling every --poll-interval seconds
        #[arg(long)]
        watch: bool,

        /// Polling interval in seconds when using --watch (default: 5)
        #[arg(long, default_value = "5")]
        poll_interval: u64,

        /// Timeout in seconds for --watch (0 = no timeout, default: 1800)
        #[arg(long, default_value = "1800")]
        watch_timeout: u64,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long, default_value = "US")]
        region: String,

        /// Force re-translation by deleting existing manifest.
        /// Note: Prior versions always forced re-translation. The default is now
        /// to preserve existing manifests.
        #[arg(long)]
        force: bool,

        /// Dispatch to a serverless Fly.io machine instead of running locally
        #[arg(long)]
        serverless: bool,

        /// Send a Slack notification when the job completes (requires swarm.toml)
        #[arg(long)]
        notify: bool,
    },

    /// Check translation status
    Status {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Wait for translation to complete
        #[arg(short, long)]
        wait: bool,
    },

    /// Get translation manifest (available derivatives)
    Manifest {
        /// Base64-encoded URN of the source file
        urn: String,
    },

    /// List downloadable derivatives
    Derivatives {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Filter by format (obj, stl, step, etc.)
        #[arg(short, long)]
        format: Option<String>,
    },

    /// Download translated derivatives
    Download {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Download by format (obj, stl, step, etc.) - downloads all matching
        #[arg(short, long)]
        format: Option<String>,

        /// Download specific derivative by GUID
        #[arg(short, long)]
        guid: Option<String>,

        /// Output directory (defaults to current directory)
        #[arg(long = "out-dir")]
        out_dir: Option<PathBuf>,

        /// Download all available derivatives
        #[arg(short, long)]
        all: bool,
    },

    /// List model views/viewables (metadata)
    Metadata {
        /// Base64-encoded URN of the source file
        urn: String,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Get object tree hierarchy for a model view
    Tree {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Model view GUID (from metadata command)
        guid: String,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Get properties for a model view
    Properties {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Model view GUID (from metadata command)
        guid: String,

        /// Filter by object ID
        #[arg(long = "object-id")]
        object_id: Option<i64>,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Query specific properties by object IDs
    #[command(name = "query-properties")]
    QueryProperties {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Model view GUID (from metadata command)
        guid: String,

        /// Comma-separated object IDs to filter (e.g., "1,2,3")
        #[arg(short, long)]
        filter: String,

        /// Comma-separated field names to return
        #[arg(long)]
        fields: Option<String>,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Manage translation presets
    #[command(subcommand)]
    Preset(PresetCommands),
}

#[derive(Debug, Subcommand)]
pub enum PresetCommands {
    /// List available translation presets
    List,

    /// Show preset details
    Show {
        /// Preset name
        name: String,
    },

    /// Create a new preset
    Create {
        /// Preset name
        name: String,
        /// Output format (svf2, obj, stl, step, etc.)
        #[arg(short, long)]
        format: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete a preset
    Delete {
        /// Preset name
        name: String,
    },

    /// Use a preset to start translation
    Use {
        /// Base64-encoded URN of the source file
        urn: String,
        /// Preset name
        preset: String,
    },
}

impl TranslateCommands {
    pub async fn execute(
        self,
        client: &DerivativeClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            TranslateCommands::Start {
                urn,
                format,
                root_filename,
                wait,
                watch,
                poll_interval,
                watch_timeout,
                region,
                force,
                serverless,
                notify,
            } => {
                if serverless {
                    translations::start_serverless(
                        urn,
                        format,
                        root_filename,
                        region,
                        force,
                        wait,
                        notify,
                        output_format,
                    )
                    .await
                } else {
                    translations::start_translation(
                        client,
                        urn,
                        format,
                        root_filename,
                        wait,
                        watch,
                        poll_interval,
                        watch_timeout,
                        output_format,
                        region,
                        force,
                    )
                    .await
                }
            }
            TranslateCommands::Status { urn, wait } => {
                translations::check_status(client, &urn, wait, output_format).await
            }
            TranslateCommands::Manifest { urn } => {
                translations::show_manifest(client, &urn, output_format).await
            }
            TranslateCommands::Derivatives { urn, format } => {
                translations::list_derivatives(client, &urn, format, output_format).await
            }
            TranslateCommands::Download {
                urn,
                format,
                guid,
                out_dir,
                all,
            } => {
                translations::download_derivatives(
                    client,
                    &urn,
                    format,
                    guid,
                    out_dir,
                    all,
                    output_format,
                )
                .await
            }
            TranslateCommands::Metadata {
                urn,
                region,
                output,
            } => metadata::show_metadata(client, &urn, region, &output).await,
            TranslateCommands::Tree {
                urn,
                guid,
                region,
                output,
            } => metadata::show_object_tree(client, &urn, &guid, region, &output).await,
            TranslateCommands::Properties {
                urn,
                guid,
                object_id,
                region,
                output,
            } => metadata::show_properties(client, &urn, &guid, object_id, region, &output).await,
            TranslateCommands::QueryProperties {
                urn,
                guid,
                filter,
                fields,
                region,
                output,
            } => {
                metadata::query_properties(client, &urn, &guid, &filter, fields, region, &output)
                    .await
            }
            TranslateCommands::Preset(cmd) => cmd.execute(client, output_format).await,
        }
    }
}

impl PresetCommands {
    pub async fn execute(
        self,
        client: &DerivativeClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            PresetCommands::List => presets::list_presets(output_format),
            PresetCommands::Show { name } => presets::show_preset(&name, output_format),
            PresetCommands::Create {
                name,
                format,
                description,
            } => presets::create_preset(&name, &format, description, output_format),
            PresetCommands::Delete { name } => presets::delete_preset(&name, output_format),
            PresetCommands::Use { urn, preset } => {
                presets::use_preset(client, &urn, &preset, output_format).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::translations::TRANSLATE_POLL_TIMEOUT;
    use std::time::Duration;

    #[test]
    fn test_translate_poll_timeout_is_two_hours() {
        assert_eq!(TRANSLATE_POLL_TIMEOUT, Duration::from_secs(7200));
        assert_eq!(TRANSLATE_POLL_TIMEOUT.as_secs() / 3600, 2);
    }
}
