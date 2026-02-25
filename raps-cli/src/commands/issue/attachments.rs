// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Issue attachment operations: list attachments

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::IssuesClient;

use super::truncate_str;

#[derive(Serialize)]
pub(super) struct AttachmentOutput {
    id: String,
    name: String,
    urn: Option<String>,
    created_at: Option<String>,
    created_by: Option<String>,
}

pub(super) async fn list_attachments(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching attachments...".dimmed());
    }

    let attachments = client
        .list_attachments(project_id, issue_id)
        .await
        .context(format!(
            "Failed to list attachments for issue '{}'",
            issue_id
        ))?;

    let outputs: Vec<AttachmentOutput> = attachments
        .iter()
        .map(|a| AttachmentOutput {
            id: a.id.clone(),
            name: a.name.clone(),
            urn: a.urn.clone(),
            created_at: a.created_at.clone(),
            created_by: a.created_by.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No attachments found.".yellow()),
            _ => output_format.write(&Vec::<AttachmentOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Attachments:".bold());
            println!("{}", "─".repeat(80));
            println!(
                "{:<40} {:<20} {}",
                "Name".bold(),
                "Created".bold(),
                "ID".bold()
            );
            println!("{}", "─".repeat(80));

            for attachment in &outputs {
                let created = attachment.created_at.as_deref().unwrap_or("-");
                println!(
                    "{:<40} {:<20} {}",
                    truncate_str(&attachment.name, 40).cyan(),
                    created.dimmed(),
                    attachment.id.dimmed()
                );
            }

            println!("{}", "─".repeat(80));
        }
        _ => output_format.write(&outputs)?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_output_serialization() {
        let output = AttachmentOutput {
            id: "att-456".to_string(),
            name: "photo.jpg".to_string(),
            urn: Some("urn:adsk.objects:os.object:bucket/photo.jpg".to_string()),
            created_at: Some("2025-02-01T12:00:00Z".to_string()),
            created_by: None,
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"id\":\"att-456\""));
        assert!(json.contains("\"name\":\"photo.jpg\""));
        assert!(json.contains("\"urn\""));
        // created_by is None so should still be present as null in default serialization
        assert!(json.contains("\"created_by\":null"));
    }
}
