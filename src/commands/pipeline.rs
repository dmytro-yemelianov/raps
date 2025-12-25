//! Pipeline execution commands
//!
//! Run multiple CLI commands from a YAML or JSON pipeline file.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum PipelineCommands {
    /// Run a pipeline from a YAML or JSON file
    Run {
        /// Path to pipeline file
        file: PathBuf,

        /// Continue on error
        #[arg(short, long)]
        continue_on_error: bool,

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
        #[arg(short, long, default_value = "pipeline.yaml")]
        output: PathBuf,
    },
}

/// Pipeline definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pipeline {
    /// Pipeline name
    pub name: String,
    /// Pipeline description
    #[serde(default)]
    pub description: Option<String>,
    /// Variables for substitution
    #[serde(default)]
    pub variables: std::collections::HashMap<String, String>,
    /// Pipeline steps
    pub steps: Vec<PipelineStep>,
}

/// Single step in a pipeline
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipelineStep {
    /// Step name
    pub name: String,
    /// Command to execute (raps subcommand, e.g., "bucket list")
    pub command: String,
    /// Continue on failure
    #[serde(default)]
    pub continue_on_error: bool,
    /// Condition to check before running
    #[serde(default)]
    pub condition: Option<String>,
}

impl PipelineCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            PipelineCommands::Run {
                file,
                continue_on_error,
                dry_run,
            } => run_pipeline(&file, continue_on_error, dry_run, output_format).await,
            PipelineCommands::Validate { file } => validate_pipeline(&file, output_format),
            PipelineCommands::Sample { output } => generate_sample(&output, output_format),
        }
    }
}

fn load_pipeline(file: &PathBuf) -> Result<Pipeline> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read pipeline file: {}", file.display()))?;

    let pipeline: Pipeline = if file
        .extension()
        .map(|e| e == "yaml" || e == "yml")
        .unwrap_or(false)
    {
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
    global_continue_on_error: bool,
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
            println!("  {} {}", "Command:".dimmed(), step.command.cyan());
        }

        // Check condition if specified
        if let Some(ref condition) = step.condition {
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
                println!("  {} Would execute: raps {}", "→".dimmed(), step.command);
            }
            passed += 1;
            continue;
        }

        // Substitute variables in command
        let mut command = step.command.clone();
        for (key, value) in &pipeline.variables {
            command = command.replace(&format!("${{{}}}", key), value);
            command = command.replace(&format!("${}", key), value);
        }

        // Execute the command
        let result = execute_raps_command(&command);

        match result {
            Ok(exit_code) if exit_code == 0 => {
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

                if !step.continue_on_error && !global_continue_on_error {
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

                if !step.continue_on_error && !global_continue_on_error {
                    anyhow::bail!("Pipeline aborted at step '{}': {}", step.name, e);
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

    let result = PipelineResult {
        success: failed == 0,
        passed,
        failed,
        skipped,
    };

    if !matches!(output_format, OutputFormat::Table) {
        output_format.write(&result)?;
    }

    if failed > 0 {
        anyhow::bail!("Pipeline completed with {} failed step(s)", failed);
    }

    Ok(())
}

fn execute_raps_command(command: &str) -> Result<i32> {
    // Get the current executable path
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;

    // Split command into args
    let args: Vec<&str> = command.split_whitespace().collect();

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
        if step.command.is_empty() {
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
    let sample = Pipeline {
        name: "Sample Pipeline".to_string(),
        description: Some("Example pipeline demonstrating raps automation".to_string()),
        variables: [
            ("BUCKET".to_string(), "my-bucket".to_string()),
            ("PROJECT_ID".to_string(), "12345".to_string()),
        ]
        .into_iter()
        .collect(),
        steps: vec![
            PipelineStep {
                name: "List buckets".to_string(),
                command: "bucket list".to_string(),
                continue_on_error: false,
                condition: None,
            },
            PipelineStep {
                name: "Create bucket".to_string(),
                command: "bucket create ${BUCKET}".to_string(),
                continue_on_error: true,
                condition: None,
            },
            PipelineStep {
                name: "List objects".to_string(),
                command: "object list ${BUCKET}".to_string(),
                continue_on_error: false,
                condition: None,
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
    continue_on_error: true
"#;

        let pipeline: Pipeline = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(pipeline.name, "Test Pipeline");
        assert_eq!(pipeline.steps.len(), 2);
        assert_eq!(
            pipeline.variables.get("BUCKET"),
            Some(&"test-bucket".to_string())
        );
        assert!(!pipeline.steps[0].continue_on_error);
        assert!(pipeline.steps[1].continue_on_error);
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
        let step: PipelineStep = serde_yaml::from_str(yaml).unwrap();
        assert!(!step.continue_on_error);
        assert!(step.condition.is_none());
    }
}
