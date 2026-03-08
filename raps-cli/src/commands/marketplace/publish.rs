// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Publish command handlers: init, package, publish, review

use anyhow::Result;
use colored::Colorize;

use crate::output::OutputFormat;

use super::{InitArgs, PackageArgs, PublishArgs, ReviewArgs};

pub(super) async fn init(args: InitArgs, output_format: OutputFormat) -> Result<()> {
    // Plugin publishing scaffolding — not yet implemented.
    let name = args.name.unwrap_or_else(|| "my-plugin".to_string());
    let author = args.author.unwrap_or_else(|| "unknown".to_string());

    match output_format {
        OutputFormat::Table => {
            println!("{} Plugin init for '{}' by '{}' — not yet implemented.", "!".yellow().bold(), name, author);
            println!("  Plugin publishing will be available in a future release.");
        }
        _ => {
            output_format.write(&serde_json::json!({
                "status": "not_implemented",
                "name": name,
                "author": author
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn package(args: PackageArgs, output_format: OutputFormat) -> Result<()> {
    // Plugin packaging — not yet implemented.
    match output_format {
        OutputFormat::Table => {
            println!("{} Plugin packaging for '{}' — not yet implemented.", "!".yellow().bold(), args.dir);
            println!("  Plugin publishing will be available in a future release.");
        }
        _ => {
            output_format.write(&serde_json::json!({
                "status": "not_implemented",
                "dir": args.dir
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn publish(args: PublishArgs, output_format: OutputFormat) -> Result<()> {
    // Plugin publishing — not yet implemented.
    match output_format {
        OutputFormat::Table => {
            println!("{} Plugin publishing for '{}' — not yet implemented.", "!".yellow().bold(), args.path);
            println!("  Plugin publishing will be available in a future release.");
        }
        _ => {
            output_format.write(&serde_json::json!({
                "status": "not_implemented",
                "path": args.path
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn review(args: ReviewArgs, output_format: OutputFormat) -> Result<()> {
    // Plugin review submission — not yet implemented.
    match output_format {
        OutputFormat::Table => {
            println!("{} Review submission for '{}' — not yet implemented.", "!".yellow().bold(), args.name);
            println!("  Plugin reviews will be available in a future release.");
        }
        _ => {
            output_format.write(&serde_json::json!({
                "status": "not_implemented",
                "plugin": args.name
            }))?;
        }
    }

    Ok(())
}
