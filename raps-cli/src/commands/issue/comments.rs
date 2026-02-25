// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Issue comment operations: list, add, delete

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::IssuesClient;

use super::truncate_str;

#[derive(Serialize)]
pub(super) struct CommentOutput {
    id: String,
    body: String,
    created_at: Option<String>,
    created_by: Option<String>,
}

pub(super) async fn list_comments(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching comments...".dimmed());
    }

    let comments = client
        .list_comments(project_id, issue_id)
        .await
        .context(format!("Failed to list comments for issue '{}'", issue_id))?;

    let outputs: Vec<CommentOutput> = comments
        .iter()
        .map(|c| CommentOutput {
            id: c.id.clone(),
            body: c.body.clone(),
            created_at: c.created_at.clone(),
            created_by: c.created_by.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No comments found.".yellow()),
            _ => output_format.write(&Vec::<CommentOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Comments:".bold());
            println!("{}", "─".repeat(80));

            for comment in &outputs {
                let created = comment.created_at.as_deref().unwrap_or("-");
                let author = comment.created_by.as_deref().unwrap_or("-");

                println!("{} {}", "ID:".bold(), comment.id.dimmed());
                println!("{} {}", "Author:".bold(), author);
                println!("{} {}", "Created:".bold(), created.dimmed());
                println!("{}", comment.body);
                println!("{}", "─".repeat(80));
            }
        }
        _ => output_format.write(&outputs)?,
    }

    Ok(())
}

pub(super) async fn add_comment(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    body: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Adding comment...".dimmed());
    }

    let comment = client
        .add_comment(project_id, issue_id, body)
        .await
        .context("Failed to add comment")?;

    #[derive(Serialize)]
    struct AddCommentOutput {
        success: bool,
        id: String,
        body: String,
    }

    let output = AddCommentOutput {
        success: true,
        id: comment.id.clone(),
        body: comment.body.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Comment added!", "✓".green().bold());
            println!("  {} {}", "ID:".bold(), output.id);
            println!("  {} {}", "Body:".bold(), truncate_str(&output.body, 50));
        }
        _ => output_format.write(&output)?,
    }

    Ok(())
}

pub(super) async fn delete_comment(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    comment_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting comment...".dimmed());
    }

    client
        .delete_comment(project_id, issue_id, comment_id)
        .await
        .context(format!("Failed to delete comment '{}'", comment_id))?;

    #[derive(Serialize)]
    struct DeleteCommentOutput {
        success: bool,
        comment_id: String,
        message: String,
    }

    let output = DeleteCommentOutput {
        success: true,
        comment_id: comment_id.to_string(),
        message: "Comment deleted successfully".to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
        }
        _ => output_format.write(&output)?,
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_output_serialization() {
        let output = CommentOutput {
            id: "comment-123".to_string(),
            body: "This is a test comment".to_string(),
            created_at: Some("2025-01-15T10:00:00Z".to_string()),
            created_by: Some("user@example.com".to_string()),
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"id\":\"comment-123\""));
        assert!(json.contains("\"body\":\"This is a test comment\""));
        assert!(json.contains("\"created_at\":\"2025-01-15T10:00:00Z\""));
        assert!(json.contains("\"created_by\":\"user@example.com\""));
    }
}
