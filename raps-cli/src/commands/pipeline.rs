// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Pipeline execution commands
//!
//! Run multiple CLI commands from a YAML or JSON pipeline file.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::output::OutputFormat;
// use raps_kernel::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum PipelineCommands {
    /// Run a pipeline from a YAML or JSON file (use `-` for stdin, parsed as YAML)
    Run {
        /// Path to pipeline file (use `-` for stdin)
        file: PathBuf,

        /// Ignore step failures and continue
        #[arg(short, long)]
        ignore_failure: bool,

        /// Dry run (show commands without executing)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Validate a pipeline file
    Validate {
        /// Path to pipeline file
        file: PathBuf,
    },

    /// Generate a sample pipeline file
    Sample {
        /// Output file path
        #[arg(long = "out-file", default_value = "pipeline.yaml")]
        out_file: PathBuf,
    },
}

/// Pipeline definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pipeline {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub variables: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub defaults: PipelineDefaults,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PipelineDefaults {
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_backoff")]
    pub backoff: BackoffStrategy,
    #[serde(default = "default_delay")]
    pub delay: String,
    #[serde(default)]
    pub on: Vec<String>,
}

fn default_max_attempts() -> u32 { 3 }
fn default_backoff() -> BackoffStrategy { BackoffStrategy::Fixed }
fn default_delay() -> String { "5s".to_string() }

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackoffStrategy {
    #[default]
    Fixed,
    Exponential,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Step {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub parallel: Option<Vec<Step>>,
    #[serde(default)]
    pub for_each: Option<ForEachConfig>,
    #[serde(default)]
    pub steps: Option<Vec<Step>>,
    #[serde(default, rename = "if")]
    pub if_expr: Option<String>,
    #[serde(default)]
    pub unless: Option<String>,
    #[serde(default)]
    pub ignore_failure: bool,
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub on_failure: Option<Vec<Step>>,
    #[serde(default)]
    pub max_concurrency: Option<usize>,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            name: String::new(),
            id: None,
            command: None,
            parallel: None,
            for_each: None,
            steps: None,
            if_expr: None,
            unless: None,
            ignore_failure: false,
            retry: None,
            timeout: None,
            on_failure: None,
            max_concurrency: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForEachConfig {
    pub var: String,
    #[serde(rename = "in")]
    pub items: Vec<String>,
    #[serde(default)]
    pub parallel: bool,
    #[serde(default)]
    pub max_concurrency: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub exit_code: i32,
}

type StepContext = Arc<Mutex<HashMap<String, StepResult>>>;

impl PipelineCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            PipelineCommands::Run {
                file,
                ignore_failure,
                dry_run,
            } => run_pipeline(&file, ignore_failure, dry_run, output_format).await,
            PipelineCommands::Validate { file } => validate_pipeline(&file, output_format),
            PipelineCommands::Sample { out_file } => generate_sample(&out_file, output_format),
        }
    }
}

fn load_pipeline(file: &PathBuf) -> Result<Pipeline> {
    let content = if file.as_os_str() == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut buf)
            .context("Failed to read pipeline from stdin")?;
        buf
    } else {
        std::fs::read_to_string(file)
            .with_context(|| format!("Failed to read pipeline file: {}", file.display()))?
    };

    // Stdin defaults to YAML; files use extension to determine format
    let is_yaml = file.as_os_str() == "-"
        || file
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false);

    let pipeline: Pipeline = if is_yaml {
        serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML pipeline: {}", file.display()))?
    } else {
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON pipeline: {}", file.display()))?
    };

    Ok(pipeline)
}

async fn run_pipeline(
    file: &PathBuf,
    global_ignore_failure: bool,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let pipeline = load_pipeline(file)?;

    if output_format.supports_colors() {
        println!("\n{} {}", "Pipeline:".bold(), pipeline.name.cyan());
        if let Some(ref desc) = pipeline.description {
            println!("  {}", desc.dimmed());
        }
        println!("{}", "─".repeat(60));
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for (i, step) in pipeline.steps.iter().enumerate() {
        let step_num = i + 1;

        if output_format.supports_colors() {
            println!(
                "\n[{}/{}] {}",
                step_num,
                pipeline.steps.len(),
                step.name.bold()
            );
            println!("  {} {}", "Command:".dimmed(), step.command.as_deref().unwrap_or("").cyan());
        }

        // Check condition if specified
        if let Some(ref condition) = step.if_expr {
            // Simple condition parsing (e.g., "exit_code == 0")
            if !evaluate_condition(condition) {
                if output_format.supports_colors() {
                    println!("  {} Condition not met, skipping", "→".yellow());
                }
                skipped += 1;
                continue;
            }
        }

        if dry_run {
            if output_format.supports_colors() {
                println!("  {} Would execute: raps {}", "→".dimmed(), step.command.as_deref().unwrap_or(""));
            }
            passed += 1;
            continue;
        }

        // Validate and substitute variables in command
        let mut command = step.command.as_deref().unwrap_or("").to_string();
        for (key, value) in &pipeline.variables {
            // Reject shell metacharacters in variable values
            const SHELL_META: &[char] = &['|', '&', ';', '$', '`', '(', ')', '{', '}', '<', '>'];
            if value.contains(SHELL_META) {
                anyhow::bail!("Pipeline variable '{}' contains shell metacharacters", key);
            }
            command = command.replace(&format!("${{{}}}", key), value);
            command = command.replace(&format!("${}", key), value);
        }

        // Execute the command
        let result = execute_raps_command(&command);

        match result {
            Ok(0) => {
                if output_format.supports_colors() {
                    println!("  {} Success", "✓".green().bold());
                }
                passed += 1;
            }
            Ok(exit_code) => {
                if output_format.supports_colors() {
                    println!("  {} Failed (exit code: {})", "✗".red().bold(), exit_code);
                }
                failed += 1;

                if !step.ignore_failure && !global_ignore_failure {
                    anyhow::bail!(
                        "Pipeline aborted at step '{}' (exit code: {})",
                        step.name,
                        exit_code
                    );
                }
            }
            Err(e) => {
                if output_format.supports_colors() {
                    println!("  {} Error: {}", "✗".red().bold(), e);
                }
                failed += 1;

                if !step.ignore_failure && !global_ignore_failure {
                    anyhow::bail!("Pipeline aborted at step '{}': {e}", step.name);
                }
            }
        }
    }

    // Summary
    if output_format.supports_colors() {
        println!("\n{}", "─".repeat(60));
        println!("{}", "Pipeline Summary:".bold());
        println!(
            "  {} {} passed, {} {} failed, {} {} skipped",
            "✓".green(),
            passed,
            "✗".red(),
            failed,
            "→".yellow(),
            skipped
        );
    }

    #[derive(Serialize)]
    struct PipelineResult {
        success: bool,
        passed: usize,
        failed: usize,
        skipped: usize,
    }

    // If we reach here, all failures were from ignore_failure steps
    // (hard failures bail immediately above), so the pipeline succeeded.
    let result = PipelineResult {
        success: true,
        passed,
        failed,
        skipped,
    };

    if !matches!(output_format, OutputFormat::Table) {
        output_format.write(&result)?;
    }

    Ok(())
}

fn execute_raps_command(command: &str) -> Result<i32> {
    // Get the current executable path
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;

    // Split command into args (shell-aware quoting)
    let args = shlex::split(command)
        .ok_or_else(|| anyhow::anyhow!("Invalid quoting in pipeline command: {}", command))?;

    // Execute raps with the given arguments
    let output = Command::new(&exe_path)
        .args(&args)
        .output()
        .context("Failed to execute command")?;

    // Print stdout/stderr
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(output.status.code().unwrap_or(-1))
}

fn evaluate_condition(condition: &str) -> bool {
    // Simple condition evaluation
    // For now, just check if it's truthy
    let trimmed = condition.trim().to_lowercase();
    !trimmed.is_empty() && trimmed != "false" && trimmed != "0"
}

fn validate_pipeline(file: &PathBuf, output_format: OutputFormat) -> Result<()> {
    let pipeline = load_pipeline(file)?;

    #[derive(Serialize)]
    struct ValidationResult {
        valid: bool,
        name: String,
        steps_count: usize,
        warnings: Vec<String>,
    }

    let mut warnings = Vec::new();

    // Check for potential issues
    for (i, step) in pipeline.steps.iter().enumerate() {
        if step.command.as_deref().unwrap_or("").is_empty() {
            warnings.push(format!("Step {} '{}' has empty command", i + 1, step.name));
        }
    }

    let result = ValidationResult {
        valid: warnings.is_empty(),
        name: pipeline.name.clone(),
        steps_count: pipeline.steps.len(),
        warnings: warnings.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            if warnings.is_empty() {
                println!(
                    "{} Pipeline '{}' is valid!",
                    "✓".green().bold(),
                    pipeline.name
                );
                println!("  {} {} steps", "Steps:".bold(), result.steps_count);
            } else {
                println!("{} Pipeline has warnings:", "!".yellow().bold());
                for warning in &warnings {
                    println!("  {} {}", "•".yellow(), warning);
                }
            }
        }
        _ => {
            output_format.write(&result)?;
        }
    }

    Ok(())
}

fn generate_sample(output: &PathBuf, output_format: OutputFormat) -> Result<()> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let bucket_name = format!("raps-sample-{ts}");
    let sample = Pipeline {
        name: "Sample Pipeline".to_string(),
        description: Some("Example pipeline demonstrating raps automation".to_string()),
        variables: [
            ("BUCKET".to_string(), bucket_name),
            ("PROJECT_ID".to_string(), "12345".to_string()),
        ]
        .into_iter()
        .collect(),
        defaults: PipelineDefaults::default(),
        steps: vec![
            Step {
                name: "List buckets".to_string(),
                command: Some("bucket list".to_string()),
                ..Step::default()
            },
            Step {
                name: "Create bucket".to_string(),
                command: Some("bucket create -k ${BUCKET} -p transient -r US".to_string()),
                ignore_failure: true,
                ..Step::default()
            },
            Step {
                name: "List objects".to_string(),
                command: Some("object list ${BUCKET}".to_string()),
                ..Step::default()
            },
            Step {
                name: "Delete bucket".to_string(),
                command: Some("bucket delete ${BUCKET} -y".to_string()),
                ignore_failure: true,
                ..Step::default()
            },
        ],
    };

    let content = if output.extension().map(|e| e == "json").unwrap_or(false) {
        serde_json::to_string_pretty(&sample)?
    } else {
        serde_yaml::to_string(&sample)?
    };

    std::fs::write(output, &content)
        .with_context(|| format!("Failed to write sample pipeline to {}", output.display()))?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Sample pipeline written to {}",
                "✓".green().bold(),
                output.display().to_string().cyan()
            );
        }
        _ => {
            #[derive(Serialize)]
            struct SampleOutput {
                success: bool,
                path: String,
            }
            output_format.write(&SampleOutput {
                success: true,
                path: output.display().to_string(),
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_deserialization_yaml() {
        let yaml = r#"
name: Test Pipeline
description: A test pipeline
variables:
  BUCKET: test-bucket
steps:
  - name: Step 1
    command: bucket list
  - name: Step 2
    command: object list ${BUCKET}
    ignore_failure: true
"#;

        let pipeline: Pipeline = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(pipeline.name, "Test Pipeline");
        assert_eq!(pipeline.steps.len(), 2);
        assert_eq!(
            pipeline.variables.get("BUCKET"),
            Some(&"test-bucket".to_string())
        );
        assert!(!pipeline.steps[0].ignore_failure);
        assert!(pipeline.steps[1].ignore_failure);
    }

    #[test]
    fn test_pipeline_deserialization_json() {
        let json = r#"{
            "name": "Test Pipeline",
            "steps": [
                {"name": "Step 1", "command": "bucket list"}
            ]
        }"#;

        let pipeline: Pipeline = serde_json::from_str(json).unwrap();
        assert_eq!(pipeline.name, "Test Pipeline");
        assert_eq!(pipeline.steps.len(), 1);
    }

    #[test]
    fn test_evaluate_condition_truthy() {
        assert!(evaluate_condition("true"));
        assert!(evaluate_condition("1"));
        assert!(evaluate_condition("yes"));
        assert!(evaluate_condition("anything"));
    }

    #[test]
    fn test_evaluate_condition_falsy() {
        assert!(!evaluate_condition("false"));
        assert!(!evaluate_condition("0"));
        assert!(!evaluate_condition(""));
        assert!(!evaluate_condition("   "));
    }

    #[test]
    fn test_pipeline_step_defaults() {
        let yaml = r#"
name: Test
command: bucket list
"#;
        let step: Step = serde_yaml::from_str(yaml).unwrap();
        assert!(!step.ignore_failure);
        assert!(step.if_expr.is_none());
    }
}
