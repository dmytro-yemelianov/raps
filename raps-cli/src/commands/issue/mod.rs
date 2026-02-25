// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Issue management commands
//!
//! Commands for managing ACC (Autodesk Construction Cloud) issues.
//! Uses the Construction Issues API: /construction/issues/v1

mod attachments;
mod comments;
mod crud;
mod transitions;

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

use crate::output::OutputFormat;
use raps_acc::IssuesClient;

#[derive(Debug, Subcommand)]
pub enum IssueCommands {
    /// List issues in a project
    List {
        /// Project ID (without "b." prefix used by Data Management API)
        project_id: String,

        /// Filter by status (open, closed, etc.)
        #[arg(short, long)]
        status: Option<String>,

        /// Only show issues created after this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
    },

    /// Create a new issue
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,

        /// Issue title
        #[arg(short, long)]
        title: Option<String>,

        /// Issue description
        #[arg(short, long)]
        description: Option<String>,

        /// Create issues from CSV file or stdin (columns: title, description, status; use `-` for stdin)
        #[arg(long, value_name = "FILE")]
        from_csv: Option<PathBuf>,
    },

    /// Update an issue
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,

        /// New status
        #[arg(short, long)]
        status: Option<String>,

        /// New title
        #[arg(short, long)]
        title: Option<String>,
    },

    /// List issue types (categories) for a project
    Types {
        /// Project ID (without "b." prefix)
        project_id: String,
    },

    /// Manage issue comments
    #[command(subcommand)]
    Comment(CommentCommands),

    /// List attachments for an issue
    Attachments {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
    },

    /// Transition an issue to a new status
    Transition {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
        /// Target status (open, answered, closed, etc.)
        #[arg(short, long)]
        to: Option<String>,
    },

    /// Delete an issue
    Delete {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum CommentCommands {
    /// List comments on an issue
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
    },

    /// Add a comment to an issue
    Add {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
        /// Comment body
        #[arg(short, long)]
        body: String,
    },

    /// Delete a comment from an issue
    Delete {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
        /// Comment ID to delete
        comment_id: String,
    },
}

impl IssueCommands {
    pub async fn execute(self, client: &IssuesClient, output_format: OutputFormat) -> Result<()> {
        match self {
            IssueCommands::List {
                project_id,
                status,
                since,
            } => crud::list_issues(client, &project_id, status, since, output_format).await,
            IssueCommands::Create {
                project_id,
                title,
                description,
                from_csv,
            } => {
                crud::create_issue(client, &project_id, title, description, from_csv, output_format)
                    .await
            }
            IssueCommands::Update {
                project_id,
                issue_id,
                status,
                title,
            } => {
                crud::update_issue(client, &project_id, &issue_id, status, title, output_format)
                    .await
            }
            IssueCommands::Types { project_id } => {
                crud::list_issue_types(client, &project_id, output_format).await
            }
            IssueCommands::Comment(cmd) => cmd.execute(client, output_format).await,
            IssueCommands::Attachments {
                project_id,
                issue_id,
            } => attachments::list_attachments(client, &project_id, &issue_id, output_format).await,
            IssueCommands::Transition {
                project_id,
                issue_id,
                to,
            } => {
                transitions::transition_issue(client, &project_id, &issue_id, to, output_format)
                    .await
            }
            IssueCommands::Delete {
                project_id,
                issue_id,
            } => crud::delete_issue(client, &project_id, &issue_id, output_format).await,
        }
    }
}

impl CommentCommands {
    pub async fn execute(self, client: &IssuesClient, output_format: OutputFormat) -> Result<()> {
        match self {
            CommentCommands::List {
                project_id,
                issue_id,
            } => comments::list_comments(client, &project_id, &issue_id, output_format).await,
            CommentCommands::Add {
                project_id,
                issue_id,
                body,
            } => {
                comments::add_comment(client, &project_id, &issue_id, &body, output_format).await
            }
            CommentCommands::Delete {
                project_id,
                issue_id,
                comment_id,
            } => {
                comments::delete_comment(client, &project_id, &issue_id, &comment_id, output_format)
                    .await
            }
        }
    }
}

/// Truncate string with ellipsis
pub(super) fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_long() {
        let result = truncate_str("hello world", 8);
        assert_eq!(result, "hello...");
        assert_eq!(result.len(), 8);
    }
}
