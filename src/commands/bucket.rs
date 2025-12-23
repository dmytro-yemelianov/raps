//! Bucket management commands
//! 
//! Commands for creating, listing, and deleting OSS buckets.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};

use crate::api::{OssClient, oss::{Region, RetentionPolicy}};

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
    pub async fn execute(self, client: &OssClient) -> Result<()> {
        match self {
            BucketCommands::Create { key, policy, region } => {
                create_bucket(client, key, policy, region).await
            }
            BucketCommands::List => {
                list_buckets(client).await
            }
            BucketCommands::Info { bucket_key } => {
                bucket_info(client, &bucket_key).await
            }
            BucketCommands::Delete { bucket_key, yes } => {
                delete_bucket(client, bucket_key, yes).await
            }
        }
    }
}

async fn create_bucket(
    client: &OssClient,
    key: Option<String>,
    policy: Option<String>,
    region: Option<String>,
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
            println!("{}", "Note: Bucket keys must be globally unique across all APS applications.".yellow());
            println!("{}", format!("Suggestion: Use a prefix like '{}-yourname'", suggested_prefix).dimmed());
            
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
        Some(r) => {
            match r.to_uppercase().as_str() {
                "US" => Region::US,
                "EMEA" => Region::EMEA,
                _ => anyhow::bail!("Invalid region. Use US or EMEA."),
            }
        }
        None => {
            let regions = Region::all();
            let selection = Select::new()
                .with_prompt("Select region")
                .items(&regions)
                .default(0)
                .interact()?;
            regions[selection]
        }
    };

    // Get retention policy interactively if not provided
    let selected_policy = match policy {
        Some(p) => {
            RetentionPolicy::from_str(&p)
                .ok_or_else(|| anyhow::anyhow!("Invalid policy. Use transient, temporary, or persistent."))?
        }
        None => {
            let policies = RetentionPolicy::all();
            let policy_labels: Vec<String> = policies.iter().map(|p| {
                match p {
                    RetentionPolicy::Transient => "transient (deleted after 24 hours)".to_string(),
                    RetentionPolicy::Temporary => "temporary (deleted after 30 days)".to_string(),
                    RetentionPolicy::Persistent => "persistent (kept until deleted)".to_string(),
                }
            }).collect();
            
            let selection = Select::new()
                .with_prompt("Select retention policy")
                .items(&policy_labels)
                .default(0)
                .interact()?;
            policies[selection]
        }
    };

    println!("{}", "Creating bucket...".dimmed());

    let bucket = client.create_bucket(&bucket_key, selected_policy, selected_region).await?;

    println!("{} Bucket created successfully!", "✓".green().bold());
    println!("  {} {}", "Key:".bold(), bucket.bucket_key);
    println!("  {} {}", "Policy:".bold(), bucket.policy_key);
    println!("  {} {}", "Owner:".bold(), bucket.bucket_owner);

    Ok(())
}

async fn list_buckets(client: &OssClient) -> Result<()> {
    println!("{}", "Fetching buckets from all regions...".dimmed());

    let buckets = client.list_buckets().await?;

    if buckets.is_empty() {
        println!("{}", "No buckets found.".yellow());
        return Ok(());
    }

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

    for bucket in buckets {
        // Convert timestamp to readable date
        let created = chrono_humanize(bucket.created_date);
        let region = bucket.region.as_deref().unwrap_or("US");
        println!(
            "{:<40} {:<12} {:<8} {}",
            bucket.bucket_key.cyan(),
            bucket.policy_key,
            region.yellow(),
            created.dimmed()
        );
    }

    println!("{}", "─".repeat(90));
    Ok(())
}

async fn bucket_info(client: &OssClient, bucket_key: &str) -> Result<()> {
    println!("{}", "Fetching bucket details...".dimmed());

    let bucket = client.get_bucket_details(bucket_key).await?;

    println!("\n{}", "Bucket Details".bold());
    println!("{}", "─".repeat(60));
    
    println!("  {} {}", "Key:".bold(), bucket.bucket_key.cyan());
    println!("  {} {}", "Owner:".bold(), bucket.bucket_owner);
    println!("  {} {}", "Policy:".bold(), bucket.policy_key);
    println!("  {} {}", "Created:".bold(), chrono_humanize(bucket.created_date));
    
    if !bucket.permissions.is_empty() {
        println!("\n  {}:", "Permissions".bold());
        for perm in &bucket.permissions {
            println!("    {} {}: {}", "•".cyan(), perm.auth_id.dimmed(), perm.access);
        }
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

async fn delete_bucket(
    client: &OssClient,
    bucket_key: Option<String>,
    skip_confirm: bool,
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

            let bucket_keys: Vec<String> = buckets.iter()
                .map(|b| b.bucket_key.clone())
                .collect();

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
            .with_prompt(format!("Are you sure you want to delete bucket '{}'?", key.red()))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Deletion cancelled.".yellow());
            return Ok(());
        }
    }

    println!("{}", "Deleting bucket...".dimmed());

    client.delete_bucket(&key).await?;

    println!("{} Bucket '{}' deleted successfully!", "✓".green().bold(), key);
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

