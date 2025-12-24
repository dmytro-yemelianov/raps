//! Bucket management commands
//!
//! Commands for creating, listing, and deleting OSS buckets.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use serde::Serialize;

use crate::api::{
    oss::{Region, RetentionPolicy},
    OssClient,
};
use crate::interactive;
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum BucketCommands {
    /// Create a new bucket (interactive)
    Create {
        /// Bucket key (optional, will prompt if not provided)
        #[arg(short, long)]
        key: Option<String>,

        /// Retention policy: transient, temporary, or persistent
        #[arg(short, long)]
        policy: Option<String>,

        /// Region: US or EMEA
        #[arg(short, long)]
        region: Option<String>,
    },

    /// List all buckets
    List,

    /// Show bucket details
    Info {
        /// Bucket key
        bucket_key: String,
    },

    /// Delete a bucket
    Delete {
        /// Bucket key to delete
        bucket_key: Option<String>,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

impl BucketCommands {
    pub async fn execute(self, client: &OssClient, output_format: OutputFormat) -> Result<()> {
        match self {
            BucketCommands::Create {
                key,
                policy,
                region,
            } => create_bucket(client, key, policy, region, output_format).await,
            BucketCommands::List => list_buckets(client, output_format).await,
            BucketCommands::Info { bucket_key } => {
                bucket_info(client, &bucket_key, output_format).await
            }
            BucketCommands::Delete { bucket_key, yes } => {
                delete_bucket(client, bucket_key, yes, output_format).await
            }
        }
    }
}

/// Serializable bucket representation for output
#[derive(Debug, Serialize)]
struct BucketOutput {
    bucket_key: String,
    policy_key: String,
    bucket_owner: String,
    created_date: u64,
    created_date_human: String,
    region: String,
}

/// Serializable bucket info representation
#[derive(Debug, Serialize)]
struct BucketInfoOutput {
    bucket_key: String,
    bucket_owner: String,
    policy_key: String,
    created_date: u64,
    created_date_human: String,
    permissions: Vec<PermissionOutput>,
}

#[derive(Debug, Serialize)]
struct PermissionOutput {
    auth_id: String,
    access: String,
}

async fn create_bucket(
    client: &OssClient,
    key: Option<String>,
    policy: Option<String>,
    region: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Generate a unique prefix suggestion based on timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let suggested_prefix = format!("aps-{}", timestamp);

    // Get bucket key interactively if not provided
    let bucket_key = match key {
        Some(k) => k,
        None => {
            // In non-interactive mode, require the key
            if interactive::is_non_interactive() {
                anyhow::bail!("Bucket key is required in non-interactive mode. Use --key flag.");
            }

            println!(
                "{}",
                "Note: Bucket keys must be globally unique across all APS applications.".yellow()
            );
            println!(
                "{}",
                format!(
                    "Suggestion: Use a prefix like '{}-yourname'",
                    suggested_prefix
                )
                .dimmed()
            );

            Input::new()
                .with_prompt("Enter bucket key")
                .with_initial_text(&suggested_prefix)
                .validate_with(|input: &String| -> Result<(), &str> {
                    if input.len() < 3 {
                        Err("Bucket key must be at least 3 characters")
                    } else if input.len() > 128 {
                        Err("Bucket key must be at most 128 characters")
                    } else if !input.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.') {
                        Err("Bucket key can only contain lowercase letters, numbers, hyphens, underscores, and dots")
                    } else {
                        Ok(())
                    }
                })
                .interact_text()?
        }
    };

    // Get region interactively if not provided
    let selected_region = match region {
        Some(r) => match r.to_uppercase().as_str() {
            "US" => Region::US,
            "EMEA" => Region::EMEA,
            _ => anyhow::bail!("Invalid region. Use US or EMEA."),
        },
        None => {
            // In non-interactive mode, default to US
            if interactive::is_non_interactive() {
                Region::US
            } else {
                let regions = Region::all();
                let selection = Select::new()
                    .with_prompt("Select region")
                    .items(&regions)
                    .default(0)
                    .interact()?;
                regions[selection]
            }
        }
    };

    // Get retention policy interactively if not provided
    let selected_policy = match policy {
        Some(p) => RetentionPolicy::from_str(&p).ok_or_else(|| {
            anyhow::anyhow!("Invalid policy. Use transient, temporary, or persistent.")
        })?,
        None => {
            // In non-interactive mode, default to transient
            if interactive::is_non_interactive() {
                RetentionPolicy::Transient
            } else {
                let policies = RetentionPolicy::all();
                let policy_labels: Vec<String> = policies
                    .iter()
                    .map(|p| match p {
                        RetentionPolicy::Transient => {
                            "transient (deleted after 24 hours)".to_string()
                        }
                        RetentionPolicy::Temporary => {
                            "temporary (deleted after 30 days)".to_string()
                        }
                        RetentionPolicy::Persistent => {
                            "persistent (kept until deleted)".to_string()
                        }
                    })
                    .collect();

                let selection = Select::new()
                    .with_prompt("Select retention policy")
                    .items(&policy_labels)
                    .default(0)
                    .interact()?;
                policies[selection]
            }
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Creating bucket...".dimmed());
    }

    let bucket = client
        .create_bucket(&bucket_key, selected_policy, selected_region)
        .await?;

    let bucket_output = BucketInfoOutput {
        bucket_key: bucket.bucket_key.clone(),
        bucket_owner: bucket.bucket_owner.clone(),
        policy_key: bucket.policy_key.clone(),
        created_date: bucket.created_date,
        created_date_human: chrono_humanize(bucket.created_date),
        permissions: bucket
            .permissions
            .iter()
            .map(|p| PermissionOutput {
                auth_id: p.auth_id.clone(),
                access: p.access.clone(),
            })
            .collect(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Bucket created successfully!", "✓".green().bold());
            println!("  {} {}", "Key:".bold(), bucket.bucket_key);
            println!("  {} {}", "Policy:".bold(), bucket.policy_key);
            println!("  {} {}", "Owner:".bold(), bucket.bucket_owner);
        }
        _ => {
            output_format.write(&bucket_output)?;
        }
    }

    Ok(())
}

async fn list_buckets(client: &OssClient, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching buckets from all regions...".dimmed());
    }

    let buckets = client.list_buckets().await?;

    if buckets.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No buckets found.".yellow()),
            _ => {
                output_format.write(&Vec::<BucketOutput>::new())?;
            }
        }
        return Ok(());
    }

    let bucket_outputs: Vec<BucketOutput> = buckets
        .iter()
        .map(|b| BucketOutput {
            bucket_key: b.bucket_key.clone(),
            policy_key: b.policy_key.clone(),
            bucket_owner: String::new(), // Not available in list response
            created_date: b.created_date,
            created_date_human: chrono_humanize(b.created_date),
            region: b.region.as_deref().unwrap_or("US").to_string(),
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Buckets:".bold());
            println!("{}", "─".repeat(90));
            println!(
                "{:<40} {:<12} {:<8} {}",
                "Bucket Key".bold(),
                "Policy".bold(),
                "Region".bold(),
                "Created".bold()
            );
            println!("{}", "─".repeat(90));

            for bucket in &bucket_outputs {
                println!(
                    "{:<40} {:<12} {:<8} {}",
                    bucket.bucket_key.cyan(),
                    bucket.policy_key,
                    bucket.region.yellow(),
                    bucket.created_date_human.dimmed()
                );
            }

            println!("{}", "─".repeat(90));
        }
        _ => {
            output_format.write(&bucket_outputs)?;
        }
    }

    Ok(())
}

async fn bucket_info(
    client: &OssClient,
    bucket_key: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching bucket details...".dimmed());
    }

    let bucket = client.get_bucket_details(bucket_key).await?;

    let bucket_output = BucketInfoOutput {
        bucket_key: bucket.bucket_key.clone(),
        bucket_owner: bucket.bucket_owner.clone(),
        policy_key: bucket.policy_key.clone(),
        created_date: bucket.created_date,
        created_date_human: chrono_humanize(bucket.created_date),
        permissions: bucket
            .permissions
            .iter()
            .map(|p| PermissionOutput {
                auth_id: p.auth_id.clone(),
                access: p.access.clone(),
            })
            .collect(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Bucket Details".bold());
            println!("{}", "─".repeat(60));

            println!("  {} {}", "Key:".bold(), bucket.bucket_key.cyan());
            println!("  {} {}", "Owner:".bold(), bucket.bucket_owner);
            println!("  {} {}", "Policy:".bold(), bucket.policy_key);
            println!(
                "  {} {}",
                "Created:".bold(),
                bucket_output.created_date_human
            );

            if !bucket.permissions.is_empty() {
                println!("\n  {}:", "Permissions".bold());
                for perm in &bucket.permissions {
                    println!(
                        "    {} {}: {}",
                        "•".cyan(),
                        perm.auth_id.dimmed(),
                        perm.access
                    );
                }
            }

            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&bucket_output)?;
        }
    }

    Ok(())
}

async fn delete_bucket(
    client: &OssClient,
    bucket_key: Option<String>,
    skip_confirm: bool,
    output_format: OutputFormat,
) -> Result<()> {
    // Get bucket key interactively if not provided
    let key = match bucket_key {
        Some(k) => k,
        None => {
            // List buckets and let user select
            let buckets = client.list_buckets().await?;
            if buckets.is_empty() {
                println!("{}", "No buckets found to delete.".yellow());
                return Ok(());
            }

            let bucket_keys: Vec<String> = buckets.iter().map(|b| b.bucket_key.clone()).collect();

            let selection = Select::new()
                .with_prompt("Select bucket to delete")
                .items(&bucket_keys)
                .interact()?;

            bucket_keys[selection].clone()
        }
    };

    // Confirm deletion
    if !skip_confirm {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to delete bucket '{}'?",
                key.red()
            ))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Deletion cancelled.".yellow());
            return Ok(());
        }
    }

    if output_format.supports_colors() {
        println!("{}", "Deleting bucket...".dimmed());
    }

    client.delete_bucket(&key).await?;

    #[derive(Serialize)]
    struct DeleteResult {
        success: bool,
        bucket_key: String,
        message: String,
    }

    let result = DeleteResult {
        success: true,
        bucket_key: key.clone(),
        message: format!("Bucket '{}' deleted successfully!", key),
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Bucket '{}' deleted successfully!",
                "✓".green().bold(),
                key
            );
        }
        _ => {
            output_format.write(&result)?;
        }
    }

    Ok(())
}

/// Convert millisecond timestamp to human-readable format
fn chrono_humanize(timestamp_ms: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let duration = Duration::from_millis(timestamp_ms);
    let datetime = UNIX_EPOCH + duration;

    if let Ok(elapsed) = datetime.elapsed() {
        let secs = elapsed.as_secs();
        if secs < 60 {
            format!("{} seconds ago", secs)
        } else if secs < 3600 {
            format!("{} minutes ago", secs / 60)
        } else if secs < 86400 {
            format!("{} hours ago", secs / 3600)
        } else {
            format!("{} days ago", secs / 86400)
        }
    } else {
        "in the future".to_string()
    }
}
