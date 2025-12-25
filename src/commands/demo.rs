//! Demo scenarios for APS functionality demonstration
//!
//! Replaces PowerShell scripts with native Rust implementations

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use futures_util::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Semaphore;
use tokio::time::sleep;

use crate::api::derivative::OutputFormat;
use crate::api::{AuthClient, DerivativeClient, OssClient};
use crate::config::Config;

#[derive(Subcommand)]
pub enum DemoCommands {
    /// Complete bucket lifecycle demonstration
    BucketLifecycle(BucketLifecycleArgs),

    /// End-to-end model processing pipeline
    ModelPipeline(ModelPipelineArgs),

    /// Explore BIM 360/ACC hubs, projects, and folders
    DataManagement(DataManagementArgs),

    /// Batch translation of multiple model files
    BatchProcessing(BatchProcessingArgs),
}

#[derive(Parser)]
pub struct BucketLifecycleArgs {
    /// Prefix for bucket names
    #[arg(long, default_value_t = format!("demo-{}", chrono::Utc::now().timestamp_millis()))]
    prefix: String,

    /// Skip cleanup at the end
    #[arg(long)]
    skip_cleanup: bool,
}

#[derive(Parser)]
pub struct ModelPipelineArgs {
    /// Path to the model file (optional, generates synthetic if not provided)
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Bucket key (auto-generated if not provided)
    #[arg(long)]
    bucket: Option<String>,

    /// Output format
    #[arg(long, default_value = "svf2")]
    format: String,

    /// Keep bucket after completion
    #[arg(long)]
    keep_bucket: bool,
}

#[derive(Parser)]
pub struct DataManagementArgs {
    /// Non-interactive mode
    #[arg(long)]
    non_interactive: bool,

    /// Export data to JSON file
    #[arg(long)]
    export: Option<PathBuf>,
}

#[derive(Parser)]
pub struct BatchProcessingArgs {
    /// Folder containing model files (optional, generates synthetic if not provided)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Maximum parallel translations
    #[arg(long, default_value = "3")]
    max_parallel: usize,

    /// Bucket prefix
    #[arg(long)]
    bucket_prefix: Option<String>,

    /// Output format
    #[arg(long, default_value = "svf2")]
    format: String,

    /// Skip cleanup
    #[arg(long)]
    skip_cleanup: bool,
}

impl DemoCommands {
    pub async fn execute(&self, concurrency: usize) -> Result<()> {
        match self {
            DemoCommands::BucketLifecycle(args) => bucket_lifecycle(args).await,
            DemoCommands::ModelPipeline(args) => model_pipeline(args).await,
            DemoCommands::DataManagement(args) => data_management(args).await,
            DemoCommands::BatchProcessing(args) => batch_processing(args, concurrency).await,
        }
    }
}

// ============================================================================
// Bucket Lifecycle Demo
// ============================================================================

async fn bucket_lifecycle(args: &BucketLifecycleArgs) -> Result<()> {
    let config = Config::from_env()?;
    let auth = AuthClient::new(config.clone());
    let oss = OssClient::new(config.clone(), auth);

    println!("\n{}", "═".repeat(60).cyan());
    println!("{}", "       APS Bucket Lifecycle Demo".cyan().bold());
    println!("{}", "═".repeat(60).cyan());
    println!("Prefix: {}", args.prefix.dimmed());

    let mut created_buckets: Vec<String> = Vec::new();

    // Step 1: Create buckets
    println!("\n{}", "[1/5] Creating buckets...".yellow());

    let buckets = vec![
        (format!("{}-us-transient", args.prefix), "US", "transient"),
        (format!("{}-us-temporary", args.prefix), "US", "temporary"),
        (
            format!("{}-emea-persistent", args.prefix),
            "EMEA",
            "persistent",
        ),
    ];

    for (name, region, policy) in &buckets {
        print!("  Creating {} in {}...", name, region);

        let region_enum = match region.to_uppercase().as_str() {
            "EMEA" => crate::api::oss::Region::EMEA,
            _ => crate::api::oss::Region::US,
        };

        let policy_enum = crate::api::oss::RetentionPolicy::from_str(policy)
            .unwrap_or(crate::api::oss::RetentionPolicy::Transient);

        match oss.create_bucket(name, policy_enum, region_enum).await {
            Ok(_) => {
                println!(" {}", "OK".green());
                created_buckets.push(name.clone());
            }
            Err(e) => {
                if e.to_string().contains("already exists") {
                    println!(" {}", "SKIP (exists)".yellow());
                    created_buckets.push(name.clone());
                } else {
                    println!(" {}: {}", "FAILED".red(), e);
                }
            }
        }
    }

    // Step 2: List buckets
    println!("\n{}", "[2/5] Listing buckets...".yellow());
    match oss.list_buckets().await {
        Ok(buckets) => {
            println!("  Found {} buckets", buckets.len());
            for bucket in buckets.iter().take(10) {
                println!("    - {} ({})", bucket.bucket_key, bucket.policy_key);
            }
            if buckets.len() > 10 {
                println!("    ... and {} more", buckets.len() - 10);
            }
        }
        Err(e) => println!("  {}: {}", "Error".red(), e),
    }

    // Step 3: Generate and upload test files
    println!(
        "\n{}",
        "[3/5] Generating and uploading test files...".yellow()
    );

    let temp_dir = std::env::temp_dir().join("aps-demo-files");
    fs::create_dir_all(&temp_dir).await?;

    let mut test_files: Vec<PathBuf> = Vec::new();

    for i in 1..=3 {
        let file_name = format!("test-model-{}.json", i);
        let file_path = temp_dir.join(&file_name);

        let content = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "name": format!("Test Model {}", i),
            "created": chrono::Utc::now().to_rfc3339(),
            "elements": [
                { "type": "Wall", "count": rand::random::<u32>() % 400 + 100 },
                { "type": "Door", "count": rand::random::<u32>() % 80 + 20 },
                { "type": "Window", "count": rand::random::<u32>() % 120 + 30 }
            ],
            "metadata": {
                "author": "Demo Script (Rust)",
                "version": format!("1.0.{}", i)
            }
        });

        fs::write(&file_path, serde_json::to_string_pretty(&content)?).await?;
        let size = fs::metadata(&file_path).await?.len();
        println!("  Generated: {} ({} bytes)", file_name, size);
        test_files.push(file_path);
    }

    // Upload to first bucket
    if let Some(target_bucket) = created_buckets.first() {
        println!("\n  Uploading to bucket: {}", target_bucket.dimmed());

        for file_path in &test_files {
            let file_name = file_path.file_name().unwrap().to_string_lossy();
            print!("  Uploading {}...", file_name);

            match oss
                .upload_object(target_bucket, &file_name, file_path)
                .await
            {
                Ok(_) => println!(" {}", "OK".green()),
                Err(e) => println!(" {}: {}", "ERROR".red(), e),
            }
        }
    }

    // Step 4: List objects
    println!("\n{}", "[4/5] Listing objects in buckets...".yellow());
    for bucket_name in &created_buckets {
        println!("\n  Bucket: {}", bucket_name.dimmed());
        match oss.list_objects(bucket_name).await {
            Ok(objects) => {
                if objects.is_empty() {
                    println!("    (empty)");
                } else {
                    for obj in &objects {
                        println!("    - {} ({} bytes)", obj.object_key, obj.size);
                    }
                }
            }
            Err(e) => println!("    {}: {}", "Error".red(), e),
        }
    }

    // Step 5: Cleanup
    if !args.skip_cleanup {
        println!("\n{}", "[5/5] Cleaning up...".yellow());

        for bucket_name in &created_buckets {
            // Delete objects first
            print!("  Deleting objects in {}...", bucket_name);
            if let Ok(objects) = oss.list_objects(bucket_name).await {
                for obj in objects {
                    let _ = oss.delete_object(bucket_name, &obj.object_key).await;
                }
            }
            println!(" done");

            // Delete bucket
            print!("  Deleting bucket {}...", bucket_name);
            match oss.delete_bucket(bucket_name).await {
                Ok(_) => println!(" {}", "OK".green()),
                Err(_) => println!(" {}", "FAILED".yellow()),
            }
        }

        // Clean temp files
        let _ = fs::remove_dir_all(&temp_dir).await;
    } else {
        println!("\n{}", "[5/5] Cleanup skipped (--skip-cleanup)".dimmed());
    }

    println!("\n{}", "═".repeat(60).cyan());
    println!("{}", "       Demo Complete".cyan().bold());
    println!("{}", "═".repeat(60).cyan());
    println!("Created buckets: {}", created_buckets.join(", "));

    Ok(())
}

// ============================================================================
// Model Pipeline Demo
// ============================================================================

async fn model_pipeline(args: &ModelPipelineArgs) -> Result<()> {
    let config = Config::from_env()?;
    let auth = AuthClient::new(config.clone());
    let oss = OssClient::new(config.clone(), auth.clone());
    let derivative = DerivativeClient::new(config.clone(), auth);

    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║           APS Model Processing Pipeline                      ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );

    let bucket_key = args
        .bucket
        .clone()
        .unwrap_or_else(|| format!("pipeline-{}", chrono::Utc::now().timestamp_millis()));

    // Create or use file
    let file_path = if let Some(ref path) = args.file {
        path.clone()
    } else {
        println!("\nNo file specified, creating synthetic test file...");
        let temp_dir = std::env::temp_dir().join("aps-pipeline-demo");
        fs::create_dir_all(&temp_dir).await?;

        let file_path = temp_dir.join("test-cube.obj");
        let obj_content = r#"# Simple Cube OBJ
# Generated by APS Demo Pipeline (Rust)

# Vertices
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0
v -1.0 -1.0 -1.0
v  1.0 -1.0 -1.0
v  1.0  1.0 -1.0
v -1.0  1.0 -1.0

# Normals
vn  0.0  0.0  1.0
vn  0.0  0.0 -1.0
vn  0.0  1.0  0.0
vn  0.0 -1.0  0.0
vn  1.0  0.0  0.0
vn -1.0  0.0  0.0

# Faces
f 1//1 2//1 3//1 4//1
f 8//2 7//2 6//2 5//2
f 4//3 3//3 7//3 8//3
f 5//4 6//4 2//4 1//4
f 2//5 6//5 7//5 3//5
f 5//6 1//6 4//6 8//6
"#;
        fs::write(&file_path, obj_content).await?;
        println!("  Created: {}", file_path.display());
        file_path
    };

    let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
    let file_size = fs::metadata(&file_path).await?.len();

    println!(
        "\nFile: {} ({:.2} KB)",
        file_name,
        file_size as f64 / 1024.0
    );
    println!("Bucket: {}", bucket_key);
    println!("Format: {}", args.format);

    // Step 1: Create bucket
    println!("\n{}", "[1/5] Creating bucket...".yellow());
    match oss
        .create_bucket(
            &bucket_key,
            crate::api::oss::RetentionPolicy::Transient,
            crate::api::oss::Region::US,
        )
        .await
    {
        Ok(_) => println!("  Bucket created successfully"),
        Err(e) => {
            if e.to_string().contains("already exists") {
                println!("  Bucket already exists, continuing...");
            } else {
                println!("  Warning: {}", e);
            }
        }
    }

    // Step 2: Upload file
    println!("\n{}", "[2/5] Uploading file...".yellow());
    let upload_start = Instant::now();
    oss.upload_object(&bucket_key, &file_name, &file_path)
        .await?;
    println!(
        "  Upload completed in {:.2}s",
        upload_start.elapsed().as_secs_f64()
    );

    // Get URN
    let urn = oss.get_urn(&bucket_key, &file_name);
    println!("  URN: {}", urn.dimmed());

    // Step 3: Start translation
    println!("\n{}", "[3/5] Starting translation...".yellow());
    let output_format = OutputFormat::from_str(&args.format).unwrap_or(OutputFormat::Svf2);
    match derivative.translate(&urn, output_format, None).await {
        Ok(_) => println!("  Translation job submitted"),
        Err(e) => println!("  Translation request: {}", e),
    }

    // Step 4: Monitor progress
    println!("\n{}", "[4/5] Monitoring translation progress...".yellow());
    let start_time = Instant::now();
    let max_wait = Duration::from_secs(600); // 10 minutes

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );

    loop {
        if start_time.elapsed() > max_wait {
            pb.finish_with_message("Timeout after 10 minutes");
            break;
        }

        match derivative.get_manifest(&urn).await {
            Ok(manifest) => {
                let status = manifest.status.to_lowercase();
                if status.contains("success") || status.contains("complete") {
                    pb.finish_with_message(format!("{} Translation complete!", "✓".green()));
                    break;
                } else if status.contains("failed") {
                    pb.finish_with_message(format!("{} Translation failed", "✗".red()));
                    break;
                } else {
                    pb.set_message(format!(
                        "Status: {} ({}s)",
                        status,
                        start_time.elapsed().as_secs()
                    ));
                }
            }
            Err(_) => {
                pb.set_message(format!("Waiting... ({}s)", start_time.elapsed().as_secs()));
            }
        }

        sleep(Duration::from_secs(3)).await;
    }

    // Step 5: Get manifest (contains derivative info)
    println!("\n{}", "[5/5] Fetching manifest...".yellow());
    match derivative.get_manifest(&urn).await {
        Ok(manifest) => {
            println!("  Manifest retrieved successfully");
            println!("\n--- Manifest Preview ---");
            println!("  Status: {}", manifest.status);
            println!("  Progress: {}", manifest.progress);
            if !manifest.derivatives.is_empty() {
                println!("  Derivatives:");
                for d in manifest.derivatives.iter().take(5) {
                    println!("    - {} ({})", d.output_type, d.status);
                }
            }
        }
        Err(e) => {
            println!("  Could not retrieve manifest: {}", e);
        }
    }

    // Summary
    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║                     Pipeline Summary                          ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );
    println!("  File:    {}", file_name);
    println!("  Bucket:  {}", bucket_key);
    println!("  URN:     {}", urn);
    println!("  Format:  {}", args.format);

    // Cleanup
    if !args.keep_bucket {
        println!("\nCleaning up bucket...");
        let _ = oss.delete_object(&bucket_key, &file_name).await;
        let _ = oss.delete_bucket(&bucket_key).await;
        println!("  Cleanup complete");
    } else {
        println!("\nBucket preserved (--keep-bucket specified)");
    }

    println!("\n{}", "=== Pipeline Complete ===".cyan());

    Ok(())
}

// ============================================================================
// Data Management Demo
// ============================================================================

async fn data_management(args: &DataManagementArgs) -> Result<()> {
    let config = Config::from_env()?;
    let auth = AuthClient::new(config.clone());

    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║         BIM 360 / ACC Data Management Explorer               ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );

    // Check authentication
    println!("\n{}", "Checking authentication...".yellow());

    let token = match auth.get_3leg_token().await {
        Ok(t) => {
            println!("  {} Authenticated (3-legged)", "✓".green());
            t
        }
        Err(_) => {
            println!("  {} 3-legged authentication required", "✗".red());
            println!("  Run: raps auth login");
            return Ok(());
        }
    };

    let client = reqwest::Client::new();

    // Step 1: List Hubs
    println!("\n{}", "[1/3] Fetching Hubs...".yellow());

    let hubs_response = client
        .get(format!("{}/hubs", config.project_url()))
        .bearer_auth(&token)
        .send()
        .await?;

    let mut export_data = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "hubs": []
    });

    if hubs_response.status().is_success() {
        let hubs: serde_json::Value = hubs_response.json().await?;

        if let Some(data) = hubs.get("data").and_then(|d| d.as_array()) {
            println!("  Found {} hubs:", data.len());

            for hub in data {
                let id = hub.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let name = hub
                    .get("attributes")
                    .and_then(|a| a.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("unnamed");

                println!("    - {} ({})", name.green(), id.dimmed());

                export_data["hubs"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({
                        "id": id,
                        "name": name
                    }));
            }
        }
    } else {
        println!("  Failed to fetch hubs: {}", hubs_response.status());
    }

    // Step 2: For each hub, show projects (limited)
    println!("\n{}", "[2/3] Sample Projects...".yellow());

    if let Some(hubs) = export_data["hubs"].as_array() {
        for hub in hubs.iter().take(2) {
            if let Some(hub_id) = hub["id"].as_str() {
                println!("\n  Hub: {}", hub["name"].as_str().unwrap_or("?"));

                let projects_response = client
                    .get(format!("{}/hubs/{}/projects", config.project_url(), hub_id))
                    .bearer_auth(&token)
                    .send()
                    .await?;

                if projects_response.status().is_success() {
                    let projects: serde_json::Value = projects_response.json().await?;

                    if let Some(data) = projects.get("data").and_then(|d| d.as_array()) {
                        for project in data.iter().take(5) {
                            let name = project
                                .get("attributes")
                                .and_then(|a| a.get("name"))
                                .and_then(|n| n.as_str())
                                .unwrap_or("unnamed");
                            let id = project.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                            println!("    - {} ({})", name, id.dimmed());
                        }
                        if data.len() > 5 {
                            println!("    ... and {} more", data.len() - 5);
                        }
                    }
                }
            }
        }
    }

    // Step 3: Interactive exploration hint
    if !args.non_interactive {
        println!("\n{}", "[3/3] Interactive Exploration".yellow());
        println!("  (Use raps commands for folder navigation)");
        println!("\n  Example commands:");
        println!("    raps hub list");
        println!("    raps project list --hub-id <hub_id>");
    }

    // Export if requested
    if let Some(ref export_path) = args.export {
        println!(
            "\n{}",
            format!("Exporting data to: {}", export_path.display()).yellow()
        );
        fs::write(export_path, serde_json::to_string_pretty(&export_data)?).await?;
        println!("  Export complete");
    }

    println!("\n{}", "=== Exploration Complete ===".cyan());

    Ok(())
}

// ============================================================================
// Batch Processing Demo
// ============================================================================

async fn batch_processing(args: &BatchProcessingArgs, concurrency: usize) -> Result<()> {
    let config = Config::from_env()?;
    let auth = AuthClient::new(config.clone());
    let oss = OssClient::new(config.clone(), auth.clone());
    let derivative = DerivativeClient::new(config.clone(), auth);

    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║            APS Batch Translation Pipeline                    ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════╝".cyan()
    );

    let bucket_prefix = args
        .bucket_prefix
        .clone()
        .unwrap_or_else(|| format!("batch-{}", chrono::Utc::now().timestamp_millis()));

    // Get or create input folder
    let input_folder = if let Some(ref path) = args.input {
        path.clone()
    } else {
        println!("\nNo input folder specified, generating synthetic test files...");

        let temp_dir = std::env::temp_dir().join("aps-batch-demo");
        fs::create_dir_all(&temp_dir).await?;

        // Generate OBJ files
        let shapes = vec![
            (
                "cube",
                vec![
                    "-1 -1 1", "1 -1 1", "1 1 1", "-1 1 1", "-1 -1 -1", "1 -1 -1", "1 1 -1",
                    "-1 1 -1",
                ],
            ),
            (
                "pyramid",
                vec!["0 1 0", "-1 -1 1", "1 -1 1", "1 -1 -1", "-1 -1 -1"],
            ),
            (
                "wedge",
                vec![
                    "-1 -1 1", "1 -1 1", "1 1 1", "-1 -1 -1", "1 -1 -1", "1 1 -1",
                ],
            ),
        ];

        for (name, vertices) in shapes {
            let file_path = temp_dir.join(format!("{}.obj", name));
            let mut content = format!("# {} OBJ\n# Generated for batch demo\n\n", name);
            for v in vertices {
                content.push_str(&format!("v {}\n", v));
            }
            content.push_str("\n# Faces (simplified)\nf 1 2 3\nf 1 3 4\n");
            fs::write(&file_path, &content).await?;
            println!("  Generated: {}.obj", name);
        }

        temp_dir
    };

    // Find supported files
    let supported_extensions = vec![
        "obj", "fbx", "dwg", "dxf", "ifc", "rvt", "rfa", "nwd", "nwc", "stp", "step", "iges", "igs",
    ];

    let mut files: Vec<PathBuf> = Vec::new();
    let mut entries = fs::read_dir(&input_folder).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if supported_extensions.contains(&ext.to_string_lossy().to_lowercase().as_str()) {
                    files.push(path);
                }
            }
        }
    }

    if files.is_empty() {
        println!("\n{}", "No supported model files found.".yellow());
        println!("Supported extensions: {}", supported_extensions.join(", "));
        return Ok(());
    }

    println!("\nFound {} files to process:", files.len());
    for file in &files {
        let size = fs::metadata(file).await?.len();
        println!(
            "  - {} ({:.2} KB)",
            file.file_name().unwrap().to_string_lossy(),
            size as f64 / 1024.0
        );
    }

    // Create bucket
    println!(
        "\n{}",
        format!("[1/4] Creating bucket: {}", bucket_prefix).yellow()
    );
    let _ = oss
        .create_bucket(
            &bucket_prefix,
            crate::api::oss::RetentionPolicy::Transient,
            crate::api::oss::Region::US,
        )
        .await;
    println!("  Bucket ready");

    // Upload and translate
    println!(
        "\n{}",
        "[2/4] Uploading and starting translations...".yellow()
    );

    #[derive(Debug)]
    struct Job {
        file: String,
        urn: String,
        status: String,
        start_time: Instant,
        end_time: Option<Instant>,
    }

    let mut jobs: Vec<Job> = Vec::new();
    let output_format = OutputFormat::from_str(&args.format).unwrap_or(OutputFormat::Svf2);

    // Use the smaller of CLI concurrency or args.max_parallel
    let max_parallel = concurrency.min(args.max_parallel);
    let semaphore = Arc::new(Semaphore::new(max_parallel));
    let mut handles = Vec::new();

    println!("\n  Processing {} files with concurrency limit of {}...", files.len(), max_parallel);

    // Wrap clients in Arc for sharing across tasks
    let oss = Arc::new(oss);
    let derivative = Arc::new(derivative);

    // Process files in parallel with concurrency limit
    for file in &files {
        let file_name = file.file_name().unwrap().to_string_lossy().to_string();
        let file_path = file.clone();
        let bucket_prefix_clone = bucket_prefix.clone();
        let oss_clone = oss.clone();
        let derivative_clone = derivative.clone();
        let semaphore_clone = semaphore.clone();
        let output_format_clone = output_format;

        let handle = tokio::spawn(async move {
            // Acquire semaphore permit (blocks if limit reached)
            let _permit = semaphore_clone.acquire().await.unwrap();

            print!("  Processing: {}...", file_name);

            let result = match oss_clone.upload_object(&bucket_prefix_clone, &file_name, &file_path).await {
                Ok(_) => {
                    let urn = oss_clone.get_urn(&bucket_prefix_clone, &file_name);

                    match derivative_clone.translate(&urn, output_format_clone, None).await {
                        Ok(_) => {
                            println!(" {}", "submitted".green());
                            Ok(Job {
                                file: file_name,
                                urn,
                                status: "submitted".to_string(),
                                start_time: Instant::now(),
                                end_time: None,
                            })
                        }
                        Err(e) => {
                            println!(" {}", "translate failed".red());
                            Ok(Job {
                                file: file_name,
                                urn,
                                status: format!("translate_failed: {}", e),
                                start_time: Instant::now(),
                                end_time: Some(Instant::now()),
                            })
                        }
                    }
                }
                Err(e) => {
                    println!(" {}", "upload failed".red());
                    Ok(Job {
                        file: file_name,
                        urn: String::new(),
                        status: format!("upload_failed: {}", e),
                        start_time: Instant::now(),
                        end_time: Some(Instant::now()),
                    })
                }
            };

            // Permit is automatically released when dropped
            result
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        match handle.await {
            Ok(Ok(job)) => jobs.push(job),
            Ok(Err(e)) => {
                eprintln!("  Error processing file: {}", e);
            }
            Err(e) => {
                eprintln!("  Task panicked: {}", e);
            }
        }
    }

    // Monitor translations
    println!("\n{}", "[3/4] Monitoring translation progress...".yellow());
    let start_time = Instant::now();
    let max_wait = Duration::from_secs(900); // 15 minutes

    loop {
        if start_time.elapsed() > max_wait {
            println!("\n  Timeout after 15 minutes");
            break;
        }

        let mut pending = 0;
        let mut completed = 0;
        let mut failed = 0;

        for job in &mut jobs {
            if job.status == "submitted" {
                if let Ok(manifest) = derivative.get_manifest(&job.urn).await {
                    let status = manifest.status.to_lowercase();
                    if status.contains("success") || status.contains("complete") {
                        job.status = "complete".to_string();
                        job.end_time = Some(Instant::now());
                    } else if status.contains("failed") {
                        job.status = "failed".to_string();
                        job.end_time = Some(Instant::now());
                    }
                }
            }

            match job.status.as_str() {
                "complete" => completed += 1,
                "submitted" => pending += 1,
                _ => failed += 1,
            }
        }

        print!(
            "\r  Progress: {} complete, {} failed, {} pending ({}s)    ",
            completed,
            failed,
            pending,
            start_time.elapsed().as_secs()
        );

        if pending == 0 {
            println!();
            break;
        }

        sleep(Duration::from_secs(5)).await;
    }

    // Results summary
    println!("\n{}", "[4/4] Results Summary".yellow());
    println!("\n  ╔═══════════════════════════════════════════════════════════╗");
    println!("  ║  File                          Status      Duration       ║");
    println!("  ╠═══════════════════════════════════════════════════════════╣");

    let mut completed_count = 0;
    let mut failed_count = 0;

    for job in &jobs {
        let duration = job
            .end_time
            .map(|e| format!("{:.1}s", (e - job.start_time).as_secs_f64()))
            .unwrap_or_else(|| "-".to_string());

        let file_display = if job.file.len() > 28 {
            format!("{}...", &job.file[..25])
        } else {
            format!("{:28}", job.file)
        };

        let (status_display, color) = match job.status.as_str() {
            "complete" => {
                completed_count += 1;
                ("complete  ".to_string(), "green")
            }
            "submitted" => ("pending   ".to_string(), "yellow"),
            _ => {
                failed_count += 1;
                ("failed    ".to_string(), "red")
            }
        };

        let line = format!(
            "  ║  {}  {}  {:12}  ║",
            file_display, status_display, duration
        );
        match color {
            "green" => println!("{}", line.green()),
            "red" => println!("{}", line.red()),
            _ => println!("{}", line.yellow()),
        }
    }

    println!("  ╚═══════════════════════════════════════════════════════════╝");

    // Statistics
    let total_time = start_time.elapsed();
    println!("\n  Statistics:");
    println!("    Total files:     {}", files.len());
    println!(
        "    Completed:       {}",
        format!("{}", completed_count).green()
    );
    if failed_count > 0 {
        println!("    Failed:          {}", format!("{}", failed_count).red());
    } else {
        println!("    Failed:          0");
    }
    println!("    Total time:      {:.1}s", total_time.as_secs_f64());
    println!(
        "    Avg per file:    {:.1}s",
        total_time.as_secs_f64() / files.len().max(1) as f64
    );

    // Cleanup
    if !args.skip_cleanup {
        println!("\n{}", "Cleaning up...".yellow());
        for file in &files {
            let file_name = file.file_name().unwrap().to_string_lossy();
            let _ = oss.delete_object(&bucket_prefix, &file_name).await;
        }
        let _ = oss.delete_bucket(&bucket_prefix).await;

        // Clean temp folder if we created it
        if args.input.is_none() {
            let _ = fs::remove_dir_all(std::env::temp_dir().join("aps-batch-demo")).await;
        }

        println!("  Cleanup complete");
    }

    println!("\n{}", "=== Batch Processing Complete ===".cyan());

    Ok(())
}
