// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Pipeline execution commands
//!
//! Run multiple CLI commands from a YAML or JSON pipeline file.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::output::OutputFormat;
// use raps_kernel::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum PipelineCommands {
    /// Run a pipeline from a YAML or JSON file with v2 features: retry, timeout, conditionals, parallel steps, and for_each loops (use `-` for stdin, parsed as YAML)
    Run {
        /// Path to pipeline file (use `-` for stdin)
        file: PathBuf,

        /// Ignore step failures and continue
        #[arg(short, long)]
        ignore_failure: bool,

        /// Dry run (show commands without executing)
        #[arg(short, long)]
        dry_run: bool,

        /// Pipeline variable override (KEY=VALUE, repeatable)
        #[arg(long = "var", value_parser = parse_var_assignment)]
        var: Vec<(String, String)>,

        /// Maximum number of steps to run concurrently within a dependency level (0 = unlimited)
        #[arg(long = "max-parallel", default_value_t = 0)]
        max_parallel: usize,
        /// Resume previous run, skipping already-completed steps
        #[arg(long)]
        resume: bool,

        /// Clear saved run state and start fresh (overrides --resume)
        #[arg(long)]
        reset: bool,

        /// Clear state from this step onwards, re-run from here
        #[arg(long)]
        reset_from: Option<String>,
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

    /// Create a scheduled/triggered pipeline definition
    Create {
        /// Pipeline name
        name: String,

        /// Source URN or bucket/object path
        #[arg(long)]
        source: Option<String>,

        /// Cron schedule expression (e.g. "0 2 * * *")
        #[arg(long)]
        cron: Option<String>,

        /// Action to perform: translate, upload, extract-props, pipeline
        #[arg(long, default_value = "translate")]
        action: String,

        /// Send notifications on completion (Slack webhook URL from swarm.toml)
        #[arg(long)]
        notify: bool,

        /// Dispatch steps to serverless Fly.io machines
        #[arg(long)]
        serverless: bool,

        /// Output pipeline definition to file
        #[arg(long = "out-file", default_value = ".pipeline.yaml")]
        out_file: PathBuf,
    },

    /// Show semantic diff between two pipeline YAML/JSON files
    Diff {
        /// First pipeline file
        file1: PathBuf,

        /// Second pipeline file
        file2: PathBuf,

        /// Output format (table or json)
        #[arg(short, long, default_value = "table")]
        output: String,
    },
}

/// Pipeline definition
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize, schemars::JsonSchema)]
pub struct PipelineDefaults {
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
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

fn default_max_attempts() -> u32 {
    3
}
fn default_backoff() -> BackoffStrategy {
    BackoffStrategy::Fixed
}
fn default_delay() -> String {
    "5s".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BackoffStrategy {
    #[default]
    Fixed,
    Exponential,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, schemars::JsonSchema)]
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
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
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

// ── Idempotent run-state ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum StepRunStatus {
    Pending,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize, Deserialize)]
struct StepRunRecord {
    name: String,
    status: StepRunStatus,
    exit_code: Option<i32>,
    completed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PipelineRunState {
    pipeline_hash: String,
    pipeline_file: String,
    started_at: String,
    steps: Vec<StepRunRecord>,
}

impl PipelineRunState {
    fn state_path(pipeline_file: &std::path::Path) -> Option<std::path::PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "autodesk", "raps")?;
        let name = pipeline_file.file_stem()?.to_string_lossy();
        Some(
            dirs.cache_dir()
                .join("pipeline_runs")
                .join(format!("{}.json", name)),
        )
    }

    fn load(pipeline_file: &std::path::Path, current_hash: &str) -> Option<Self> {
        let path = Self::state_path(pipeline_file)?;
        let content = std::fs::read_to_string(&path).ok()?;
        let state: Self = serde_json::from_str(&content).ok()?;
        if state.pipeline_hash != current_hash {
            return None;
        }
        Some(state)
    }

    fn save(&self, pipeline_file: &std::path::Path) {
        if let Some(path) = Self::state_path(pipeline_file) {
            if let Some(p) = path.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            if let Ok(s) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, s);
            }
        }
    }

    fn clear(pipeline_file: &std::path::Path) {
        if let Some(path) = Self::state_path(pipeline_file) {
            let _ = std::fs::remove_file(path);
        }
    }

    fn is_completed(&self, step_name: &str) -> bool {
        self.steps
            .iter()
            .any(|s| s.name == step_name && s.status == StepRunStatus::Completed)
    }

    fn mark(&mut self, step_name: &str, status: StepRunStatus, exit_code: Option<i32>) {
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(rec) = self.steps.iter_mut().find(|s| s.name == step_name) {
            rec.status = status;
            rec.exit_code = exit_code;
            rec.completed_at = Some(now);
        } else {
            self.steps.push(StepRunRecord {
                name: step_name.to_string(),
                status,
                exit_code,
                completed_at: Some(now),
            });
        }
    }
}

fn pipeline_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(content.as_bytes()))
}

// ───────────────────────────────────────────────────────────────────────────

impl PipelineCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            PipelineCommands::Run {
                file,
                ignore_failure,
                dry_run,
                var,
                max_parallel,
                resume,
                reset,
                reset_from,
            } => {
                run_pipeline(
                    &file,
                    ignore_failure,
                    dry_run,
                    var,
                    max_parallel,
                    output_format,
                    resume,
                    reset,
                    reset_from,
                )
                .await
            }
            PipelineCommands::Validate { file } => validate_pipeline(&file, output_format).await,
            PipelineCommands::Sample { out_file } => generate_sample(&out_file, output_format),
            PipelineCommands::Create {
                name,
                source,
                cron,
                action,
                notify,
                serverless,
                out_file,
            } => create_pipeline(
                &name,
                source,
                cron,
                &action,
                notify,
                serverless,
                &out_file,
                output_format,
            ),
            PipelineCommands::Diff {
                file1,
                file2,
                output,
            } => {
                let fmt = output.parse::<OutputFormat>().unwrap_or(OutputFormat::Table);
                diff_pipelines(&file1, &file2, fmt).await
            }
        }
    }
}

async fn load_pipeline(file: &PathBuf) -> Result<Pipeline> {
    let content = if file.as_os_str() == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut buf)
            .context("Failed to read pipeline from stdin")?;
        buf
    } else {
        tokio::fs::read_to_string(file)
            .await
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

#[derive(Debug)]
enum StepOutcome {
    Success(i32),
    Failed(i32),
    Skipped,
}

fn substitute_variables(command: &str, variables: &HashMap<String, String>) -> Result<String> {
    let mut result = command.to_string();
    for (key, value) in variables {
        const SHELL_META: &[char] = &['|', '&', ';', '$', '`', '(', ')', '{', '}', '<', '>'];
        if value.contains(SHELL_META) {
            anyhow::bail!("Pipeline variable '{}' contains shell metacharacters", key);
        }
        result = result.replace(&format!("${{{}}}", key), value);
        result = result.replace(&format!("${}", key), value);
    }
    Ok(result)
}

async fn execute_command_step(
    step: &Step,
    cmd: &str,
    variables: &HashMap<String, String>,
    defaults: &PipelineDefaults,
    dry_run: bool,
    output_format: &OutputFormat,
) -> Result<StepOutcome> {
    let command = substitute_variables(cmd, variables)?;

    if dry_run {
        if output_format.supports_colors() {
            println!("  {} Would execute: raps {}", "→".dimmed(), command);
        }
        return Ok(StepOutcome::Success(0));
    }

    // Determine retry config (step overrides defaults)
    let retry = step
        .retry
        .clone()
        .or_else(|| defaults.retry.clone())
        .unwrap_or(RetryConfig {
            max_attempts: 1,
            backoff: BackoffStrategy::Fixed,
            delay: "0s".to_string(),
            on: vec![],
        });

    // Determine timeout
    let timeout_duration = step
        .timeout
        .as_deref()
        .or(defaults.timeout.as_deref())
        .map(parse_duration)
        .transpose()?;

    let result = if let Some(timeout_dur) = timeout_duration {
        match tokio::time::timeout(timeout_dur, execute_with_retry(&retry, &command)).await {
            Ok(r) => r?,
            Err(_) => {
                if output_format.supports_colors() {
                    eprintln!("  Step timed out after {}s", timeout_dur.as_secs());
                }
                return Ok(StepOutcome::Failed(-1));
            }
        }
    } else {
        execute_with_retry(&retry, &command).await?
    };

    if result == 0 {
        Ok(StepOutcome::Success(0))
    } else {
        Ok(StepOutcome::Failed(result))
    }
}

async fn execute_parallel_steps(
    steps: Vec<Step>,
    variables: HashMap<String, String>,
    context: StepContext,
    defaults: PipelineDefaults,
    output_format: OutputFormat,
    dry_run: bool,
    max_concurrency: usize,
) -> Result<StepOutcome> {
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for step in steps {
        let sem = semaphore.clone();
        let vars = variables.clone();
        let ctx = context.clone();
        let defs = defaults.clone();
        let fmt = output_format;

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            execute_step(step, vars, ctx, defs, fmt, dry_run).await
        });
        handles.push(handle);
    }

    let mut any_failed = false;
    let mut last_code = 0;

    for handle in handles {
        match handle.await {
            Ok(Ok(StepOutcome::Failed(code))) => {
                any_failed = true;
                last_code = code;
            }
            Ok(Err(e)) => return Err(e),
            Err(e) => anyhow::bail!("Parallel task panicked: {}", e),
            _ => {}
        }
    }

    if any_failed {
        Ok(StepOutcome::Failed(last_code))
    } else {
        Ok(StepOutcome::Success(0))
    }
}

#[allow(clippy::too_many_arguments)]
async fn execute_for_each(
    config: ForEachConfig,
    parent_step: Step,
    inner_steps: Vec<Step>,
    variables: HashMap<String, String>,
    context: StepContext,
    defaults: PipelineDefaults,
    output_format: OutputFormat,
    dry_run: bool,
) -> Result<StepOutcome> {
    if config.parallel {
        use tokio::sync::Semaphore;

        let max_concurrency = config.max_concurrency.unwrap_or(5);
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for item in &config.items {
            let sem = semaphore.clone();
            let mut iter_vars = variables.clone();
            iter_vars.insert(config.var.clone(), item.clone());
            let ctx = context.clone();
            let defs = defaults.clone();
            let fmt = output_format;

            let steps_to_run: Vec<Step> = if let Some(ref cmd) = parent_step.command {
                vec![Step {
                    name: format!("{} [{}={}]", parent_step.name, config.var, item),
                    command: Some(cmd.clone()),
                    ..Step::default()
                }]
            } else {
                inner_steps.clone()
            };

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                for step in steps_to_run {
                    match execute_step(
                        step.clone(),
                        iter_vars.clone(),
                        ctx.clone(),
                        defs.clone(),
                        fmt,
                        dry_run,
                    )
                    .await?
                    {
                        StepOutcome::Failed(c) if !step.ignore_failure => {
                            return Ok::<_, anyhow::Error>(StepOutcome::Failed(c));
                        }
                        _ => {}
                    }
                }
                Ok(StepOutcome::Success(0))
            });
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(Ok(StepOutcome::Failed(code))) => return Ok(StepOutcome::Failed(code)),
                Ok(Err(e)) => return Err(e),
                Err(e) => anyhow::bail!("ForEach task panicked: {}", e),
                _ => {}
            }
        }
        Ok(StepOutcome::Success(0))
    } else {
        // Sequential for_each
        for item in &config.items {
            let mut iter_vars = variables.clone();
            iter_vars.insert(config.var.clone(), item.clone());

            let steps_to_run: Vec<Step> = if let Some(ref cmd) = parent_step.command {
                vec![Step {
                    name: format!("{} [{}={}]", parent_step.name, config.var, item),
                    command: Some(cmd.clone()),
                    ..Step::default()
                }]
            } else {
                inner_steps.clone()
            };

            for step in steps_to_run {
                match execute_step(
                    step.clone(),
                    iter_vars.clone(),
                    context.clone(),
                    defaults.clone(),
                    output_format,
                    dry_run,
                )
                .await?
                {
                    StepOutcome::Failed(c) if !step.ignore_failure => {
                        return Ok(StepOutcome::Failed(c));
                    }
                    _ => {}
                }
            }
        }
        Ok(StepOutcome::Success(0))
    }
}

fn execute_step(
    step: Step,
    variables: HashMap<String, String>,
    context: StepContext,
    defaults: PipelineDefaults,
    output_format: OutputFormat,
    dry_run: bool,
) -> BoxFuture<'static, Result<StepOutcome>> {
    async move {
        // Evaluate if/unless conditions
        if let Some(ref expr) = step.if_expr
            && !eval_expression(expr, &context)?
        {
            return Ok(StepOutcome::Skipped);
        }
        if let Some(ref expr) = step.unless
            && eval_expression(expr, &context)?
        {
            return Ok(StepOutcome::Skipped);
        }

        let outcome = if let Some(ref cmd) = step.command {
            execute_command_step(&step, cmd, &variables, &defaults, dry_run, &output_format).await?
        } else if let Some(ref parallel_steps) = step.parallel {
            execute_parallel_steps(
                parallel_steps.clone(),
                variables.clone(),
                context.clone(),
                defaults.clone(),
                output_format,
                dry_run,
                step.max_concurrency.unwrap_or(10),
            )
            .await?
        } else if let Some(ref for_each) = step.for_each {
            let inner_steps = step.steps.clone().unwrap_or_default();
            execute_for_each(
                for_each.clone(),
                step.clone(),
                inner_steps,
                variables.clone(),
                context.clone(),
                defaults.clone(),
                output_format,
                dry_run,
            )
            .await?
        } else {
            anyhow::bail!("Step '{}' has no command, parallel, or for_each", step.name);
        };

        // Record result in context
        if let Some(ref id) = step.id {
            let exit_code = match &outcome {
                StepOutcome::Success(code) => *code,
                StepOutcome::Failed(code) => *code,
                StepOutcome::Skipped => 0,
            };
            context
                .lock()
                .unwrap()
                .insert(id.clone(), StepResult { exit_code });
        }

        // Run on_failure steps if step failed
        if matches!(outcome, StepOutcome::Failed(_))
            && let Some(ref failure_steps) = step.on_failure
        {
            for fs in failure_steps {
                let _ = execute_step(
                    fs.clone(),
                    variables.clone(),
                    context.clone(),
                    defaults.clone(),
                    output_format,
                    dry_run,
                )
                .await;
            }
        }

        Ok(outcome)
    }
    .boxed()
}

fn topological_sort(steps: &[Step]) -> Result<Vec<usize>> {
    use std::collections::{HashMap, VecDeque};

    let name_to_idx: HashMap<&str, usize> = steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();

    // Validate all depends_on references
    for step in steps {
        for dep in &step.depends_on {
            if !name_to_idx.contains_key(dep.as_str()) {
                anyhow::bail!(
                    "Step '{}' depends on unknown step '{}'",
                    step.name,
                    dep
                );
            }
        }
    }

    // Kahn's algorithm
    let n = steps.len();
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];

    for (i, step) in steps.iter().enumerate() {
        for dep in &step.depends_on {
            let dep_idx = name_to_idx[dep.as_str()];
            adj[dep_idx].push(i);
            in_degree[i] += 1;
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut result = Vec::with_capacity(n);

    while let Some(node) = queue.pop_front() {
        result.push(node);
        for &next in &adj[node] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push_back(next);
            }
        }
    }

    if result.len() != n {
        // Find cycle — collect remaining nodes with in_degree > 0
        let cycle_steps: Vec<&str> = (0..n)
            .filter(|&i| in_degree[i] > 0)
            .map(|i| steps[i].name.as_str())
            .collect();
        anyhow::bail!(
            "Circular dependency detected among steps: {}",
            cycle_steps.join(" → ")
        );
    }

    Ok(result)
}

/// Group topologically sorted step indices into parallel execution levels.
///
/// Level 0 contains steps with no dependencies.  Level N contains steps whose
/// all dependencies are in levels 0..N-1.  Steps at the same level have no
/// dependency between them and can safely run concurrently.
fn group_by_level(steps: &[Step], sorted_indices: &[usize]) -> Vec<Vec<usize>> {
    let mut level_of = vec![0usize; steps.len()];
    for &i in sorted_indices {
        let max_dep_level = steps[i]
            .depends_on
            .iter()
            .filter_map(|dep| steps.iter().position(|s| s.name == *dep))
            .map(|dep_idx| level_of[dep_idx])
            .max()
            .unwrap_or(0);
        level_of[i] = if steps[i].depends_on.is_empty() {
            0
        } else {
            max_dep_level + 1
        };
    }
    let max_level = level_of.iter().copied().max().unwrap_or(0);
    (0..=max_level)
        .map(|l| {
            sorted_indices
                .iter()
                .copied()
                .filter(|&i| level_of[i] == l)
                .collect()
        })
        .collect()
}

async fn run_pipeline(
    file: &PathBuf,
    global_ignore_failure: bool,
    dry_run: bool,
    var_overrides: Vec<(String, String)>,
    max_parallel: usize,
    output_format: OutputFormat,
    resume: bool,
    reset: bool,
    reset_from: Option<String>,
) -> Result<()> {
    // Read raw content for hashing (stdin pipelines are not stateful)
    let raw_content = if file.as_os_str() == "-" {
        String::new()
    } else {
        tokio::fs::read_to_string(file)
            .await
            .with_context(|| format!("Failed to read pipeline file: {}", file.display()))?
    };
    let hash = pipeline_hash(&raw_content);
    let canonical_file = file.canonicalize().unwrap_or_else(|_| file.clone());

    // Handle state reset/resume
    if reset {
        PipelineRunState::clear(&canonical_file);
    }

    let mut state: PipelineRunState = if reset {
        PipelineRunState {
            pipeline_hash: hash.clone(),
            pipeline_file: canonical_file.display().to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            steps: Vec::new(),
        }
    } else if let Some(ref from_step) = reset_from {
        // Load existing state, mark `from_step` and everything after it as Pending
        let mut s = PipelineRunState::load(&canonical_file, &hash).unwrap_or_else(|| {
            PipelineRunState {
                pipeline_hash: hash.clone(),
                pipeline_file: canonical_file.display().to_string(),
                started_at: chrono::Utc::now().to_rfc3339(),
                steps: Vec::new(),
            }
        });
        // Find the index of the reset-from step and clear from there
        let reset_idx = s.steps.iter().position(|r| &r.name == from_step);
        if let Some(idx) = reset_idx {
            for rec in s.steps[idx..].iter_mut() {
                rec.status = StepRunStatus::Pending;
                rec.exit_code = None;
                rec.completed_at = None;
            }
        }
        s.save(&canonical_file);
        s
    } else if resume {
        PipelineRunState::load(&canonical_file, &hash).unwrap_or_else(|| {
            PipelineRunState {
                pipeline_hash: hash.clone(),
                pipeline_file: canonical_file.display().to_string(),
                started_at: chrono::Utc::now().to_rfc3339(),
                steps: Vec::new(),
            }
        })
    } else {
        PipelineRunState {
            pipeline_hash: hash.clone(),
            pipeline_file: canonical_file.display().to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            steps: Vec::new(),
        }
    };

    let mut pipeline = load_pipeline(file).await?;
    apply_variable_overrides(&mut pipeline.variables, &var_overrides);
    let context: StepContext = Arc::new(Mutex::new(HashMap::new()));

    if output_format.supports_colors() {
        println!("\n{} {}", "Pipeline:".bold(), pipeline.name.cyan());
        if let Some(ref desc) = pipeline.description {
            println!("  {}", desc.dimmed());
        }
        println!("{}", "─".repeat(60));
    }

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    let execution_order = topological_sort(&pipeline.steps)?;
    let total = execution_order.len();
    let levels = group_by_level(&pipeline.steps, &execution_order);

    if dry_run && output_format.supports_colors() {
        for (level_idx, level_indices) in levels.iter().enumerate() {
            let names: Vec<&str> = level_indices
                .iter()
                .map(|&i| pipeline.steps[i].name.as_str())
                .collect();
            if level_indices.len() == 1 {
                println!("Level {} (sequential): {}", level_idx, names.join(", "));
            } else {
                println!("Level {} (parallel): {}", level_idx, names.join(", "));
            }
        }
    }

    for level_indices in &levels {
        if level_indices.len() == 1 {
            // Single step in this level — run sequentially
            let step_idx = level_indices[0];
            let step = &pipeline.steps[step_idx];
            let pos = execution_order
                .iter()
                .position(|&i| i == step_idx)
                .unwrap_or(0);

            if output_format.supports_colors() {
                println!("\n[{}/{}] {}", pos + 1, total, step.name.bold());
                if let Some(ref cmd) = step.command {
                    println!("  {} {}", "Command:".dimmed(), cmd.cyan());
                } else if step.parallel.is_some() {
                    println!("  {} parallel steps", "⫸".dimmed());
                } else if step.for_each.is_some() {
                    println!("  {} for-each loop", "⟳".dimmed());
                }
            }

            // Skip already-completed steps when resuming
            if state.is_completed(&step.name) {
                if output_format.supports_colors() {
                    println!(
                        "  {} {} (skipped — already completed)",
                        "✓".green().dimmed(),
                        step.name.dimmed()
                    );
                }
                skipped += 1;
                continue;
            }

            let outcome = execute_step(
                step.clone(),
                pipeline.variables.clone(),
                context.clone(),
                pipeline.defaults.clone(),
                output_format,
                dry_run,
            )
            .await?;

            match outcome {
                StepOutcome::Success(code) => {
                    if output_format.supports_colors() {
                        println!("  {} Success", "✓".green().bold());
                    }
                    state.mark(&step.name, StepRunStatus::Completed, Some(code));
                    state.save(&canonical_file);
                    passed += 1;
                }
                StepOutcome::Failed(code) => {
                    if output_format.supports_colors() {
                        println!("  {} Failed (exit code: {})", "✗".red().bold(), code);
                    }
                    state.mark(&step.name, StepRunStatus::Failed, Some(code));
                    state.save(&canonical_file);
                    failed += 1;
                    if !step.ignore_failure && !global_ignore_failure {
                        anyhow::bail!(
                            "Pipeline aborted at step '{}' (exit code: {})",
                            step.name,
                            code
                        );
                    }
                }
                StepOutcome::Skipped => {
                    if output_format.supports_colors() {
                        println!("  {} Skipped (condition not met)", "○".dimmed());
                    }
                    state.mark(&step.name, StepRunStatus::Skipped, None);
                    state.save(&canonical_file);
                    skipped += 1;
                }
            }
        } else {
            // Multiple independent steps — run concurrently using tokio::task::JoinSet
            use tokio::sync::Semaphore;
            use tokio::task::JoinSet;

            if output_format.supports_colors() {
                let names: Vec<&str> = level_indices
                    .iter()
                    .map(|&i| pipeline.steps[i].name.as_str())
                    .collect();
                println!("\n[parallel] {}", names.join(", "));
            }

            let semaphore: Option<Arc<Semaphore>> = if max_parallel > 0 {
                Some(Arc::new(Semaphore::new(max_parallel)))
            } else {
                None
            };

            let mut join_set: JoinSet<Result<(String, bool, StepOutcome)>> = JoinSet::new();

            for &step_idx in level_indices {
                let step = pipeline.steps[step_idx].clone();
                let vars = pipeline.variables.clone();
                let ctx = context.clone();
                let defs = pipeline.defaults.clone();
                let fmt = output_format;
                let dry = dry_run;
                let sem = semaphore.clone();
                let step_ignore = step.ignore_failure;
                let step_name = step.name.clone();

                join_set.spawn(async move {
                    let _permit = if let Some(ref s) = sem {
                        Some(s.acquire().await.expect("semaphore closed"))
                    } else {
                        None
                    };
                    let outcome = execute_step(step, vars, ctx, defs, fmt, dry).await?;
                    Ok((step_name, step_ignore, outcome))
                });
            }

            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(Ok((step_name, step_ignore, outcome))) => match outcome {
                        StepOutcome::Success(_) => {
                            if output_format.supports_colors() {
                                println!("  {} {} Success", "✓".green().bold(), step_name);
                            }
                            passed += 1;
                        }
                        StepOutcome::Failed(code) => {
                            if output_format.supports_colors() {
                                println!(
                                    "  {} {} Failed (exit code: {})",
                                    "✗".red().bold(),
                                    step_name,
                                    code
                                );
                            }
                            failed += 1;
                            if !step_ignore && !global_ignore_failure {
                                join_set.abort_all();
                                anyhow::bail!(
                                    "Pipeline aborted at step '{}' (exit code: {})",
                                    step_name,
                                    code
                                );
                            }
                        }
                        StepOutcome::Skipped => {
                            if output_format.supports_colors() {
                                println!(
                                    "  {} {} Skipped (condition not met)",
                                    "○".dimmed(),
                                    step_name
                                );
                            }
                            skipped += 1;
                        }
                    },
                    Ok(Err(e)) => return Err(e),
                    Err(e) => anyhow::bail!("Parallel step task panicked: {}", e),
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
            "○".yellow(),
            skipped
        );
    }

    #[derive(Serialize, schemars::JsonSchema)]
    struct PipelineResult {
        success: bool,
        passed: u32,
        failed: u32,
        skipped: u32,
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

    Ok(())
}

fn parse_var_assignment(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid variable assignment '{}'. Expected KEY=VALUE",
            s
        ));
    }
    let key = parts[0].trim();
    if key.is_empty() {
        return Err("Variable key cannot be empty".to_string());
    }
    Ok((key.to_string(), parts[1].to_string()))
}

fn apply_variable_overrides(
    base: &mut HashMap<String, String>,
    overrides: &[(String, String)],
) {
    for (key, value) in overrides {
        base.insert(key.clone(), value.clone());
    }
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

#[cfg(test)]
fn evaluate_condition(condition: &str) -> bool {
    // Simple condition evaluation
    // For now, just check if it's truthy
    let trimmed = condition.trim().to_lowercase();
    !trimmed.is_empty() && trimmed != "false" && trimmed != "0"
}

fn eval_expression(expr: &str, context: &StepContext) -> Result<bool> {
    let trimmed = expr.trim();

    // Check for ${{ ... }} template syntax
    if let Some(inner) = trimmed
        .strip_prefix("${{")
        .and_then(|s| s.strip_suffix("}}"))
    {
        let inner = inner.trim();
        return eval_comparison(inner, context);
    }

    // Fallback: simple truthiness
    let lower = trimmed.to_lowercase();
    Ok(!matches!(lower.as_str(), "false" | "0" | ""))
}

fn eval_comparison(expr: &str, context: &StepContext) -> Result<bool> {
    // Support: <left> || <right>
    // Parse OR first so AND has higher precedence.
    if let Some((left, right)) = expr.split_once("||") {
        return Ok(
            eval_comparison(left.trim(), context)? || eval_comparison(right.trim(), context)?
        );
    }
    // Support: <left> && <right>
    if let Some((left, right)) = expr.split_once("&&") {
        return Ok(
            eval_comparison(left.trim(), context)? && eval_comparison(right.trim(), context)?
        );
    }

    // Negation
    if let Some(inner) = expr.strip_prefix('!') {
        return Ok(!eval_comparison(inner.trim(), context)?);
    }

    // Comparison operators
    let (lhs, op, rhs) = if let Some((l, r)) = expr.split_once("!=") {
        (l.trim(), "!=", r.trim())
    } else if let Some((l, r)) = expr.split_once("==") {
        (l.trim(), "==", r.trim())
    } else {
        anyhow::bail!("Unsupported expression: {}", expr);
    };

    let lhs_val = resolve_value(lhs, context)?;
    let rhs_val: i32 = rhs
        .parse()
        .with_context(|| format!("Invalid number in expression: {}", rhs))?;

    match op {
        "==" => Ok(lhs_val == rhs_val),
        "!=" => Ok(lhs_val != rhs_val),
        _ => unreachable!(),
    }
}

fn resolve_value(path: &str, context: &StepContext) -> Result<i32> {
    // Expected format: steps.<id>.exit_code
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 3 && parts[0] == "steps" && parts[2] == "exit_code" {
        let step_id = parts[1];
        let ctx = context.lock().unwrap();
        let result = ctx
            .get(step_id)
            .ok_or_else(|| anyhow::anyhow!("Step '{}' not found in context", step_id))?;
        Ok(result.exit_code)
    } else {
        anyhow::bail!("Unknown variable path: {}", path);
    }
}

fn parse_duration(s: &str) -> Result<std::time::Duration> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty duration string");
    }
    let (num_str, suffix) = if let Some(stripped) = s.strip_suffix('s') {
        (stripped, 's')
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, 'm')
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, 'h')
    } else {
        anyhow::bail!("Duration must end with 's', 'm', or 'h': {}", s);
    };
    let num: u64 = num_str
        .parse()
        .with_context(|| format!("Invalid duration number: {}", num_str))?;
    let secs = match suffix {
        's' => num,
        'm' => num * 60,
        'h' => num * 3600,
        _ => unreachable!(),
    };
    Ok(std::time::Duration::from_secs(secs))
}

async fn execute_with_retry(config: &RetryConfig, command: &str) -> Result<i32> {
    let base_delay = parse_duration(&config.delay)?;
    let mut last_exit = -1;

    for attempt in 1..=config.max_attempts {
        match execute_raps_command(command) {
            Ok(0) => return Ok(0),
            Ok(code) => {
                last_exit = code;
                if attempt == config.max_attempts {
                    return Ok(last_exit);
                }
                let delay = match config.backoff {
                    BackoffStrategy::Fixed => base_delay,
                    BackoffStrategy::Exponential => base_delay * 2u32.pow(attempt - 1),
                };
                eprintln!(
                    "  Retry {}/{} in {}s...",
                    attempt,
                    config.max_attempts,
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                last_exit = -1;
                if attempt == config.max_attempts {
                    return Err(e);
                }
                let delay = match config.backoff {
                    BackoffStrategy::Fixed => base_delay,
                    BackoffStrategy::Exponential => base_delay * 2u32.pow(attempt - 1),
                };
                eprintln!(
                    "  Error: {}. Retry {}/{} in {}s...",
                    e,
                    attempt,
                    config.max_attempts,
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
    Ok(last_exit)
}

async fn validate_pipeline(file: &PathBuf, output_format: OutputFormat) -> Result<()> {
    let pipeline = load_pipeline(file).await?;

    #[derive(Serialize, schemars::JsonSchema)]
    struct ValidationResult {
        valid: bool,
        name: String,
        steps_count: usize,
        errors: Vec<String>,
        warnings: Vec<String>,
    }

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Validate defaults
    if let Some(ref retry) = pipeline.defaults.retry
        && retry.max_attempts < 1
    {
        errors.push("defaults.retry.max_attempts must be >= 1".to_string());
    }
    if let Some(ref timeout) = pipeline.defaults.timeout
        && parse_duration(timeout).is_err()
    {
        errors.push(format!(
            "defaults.timeout '{}' is not a valid duration (use e.g. 5s, 30m, 2h)",
            timeout
        ));
    }

    validate_steps(&pipeline.steps, &mut errors, &mut warnings, "");

    // Resolve dependency order; capture any dep errors as validation errors
    let execution_order: Option<Vec<usize>> = match topological_sort(&pipeline.steps) {
        Ok(order) => Some(order),
        Err(e) => {
            errors.push(e.to_string());
            None
        }
    };

    let result = ValidationResult {
        valid: errors.is_empty(),
        name: pipeline.name.clone(),
        steps_count: pipeline.steps.len(),
        errors: errors.clone(),
        warnings: warnings.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            if errors.is_empty() && warnings.is_empty() {
                println!(
                    "{} Pipeline '{}' is valid!",
                    "✓".green().bold(),
                    pipeline.name
                );
                println!("  {} {} steps", "Steps:".bold(), result.steps_count);
                if let Some(ref order) = execution_order {
                    let order_names: Vec<&str> = order
                        .iter()
                        .map(|&i| pipeline.steps[i].name.as_str())
                        .collect();
                    println!("  {} {}", "Execution order:".bold(), order_names.join(" → "));
                }
            } else {
                if !errors.is_empty() {
                    println!("{} Pipeline has errors:", "✗".red().bold());
                    for error in &errors {
                        println!("  {} {}", "•".red(), error);
                    }
                }
                if !warnings.is_empty() {
                    println!("{} Pipeline has warnings:", "!".yellow().bold());
                    for warning in &warnings {
                        println!("  {} {}", "•".yellow(), warning);
                    }
                }
            }
        }
        _ => {
            output_format.write(&result)?;
        }
    }

    Ok(())
}

fn validate_steps(
    steps: &[Step],
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
    prefix: &str,
) {
    for (i, step) in steps.iter().enumerate() {
        let step_label = if prefix.is_empty() {
            format!("Step {} '{}'", i + 1, step.name)
        } else {
            format!("{} > Step {} '{}'", prefix, i + 1, step.name)
        };

        // Steps must have at least one of: command, parallel, or for_each
        let has_command = step
            .command
            .as_deref()
            .map(|c| !c.is_empty())
            .unwrap_or(false);
        let has_parallel = step.parallel.is_some();
        let has_for_each = step.for_each.is_some();

        if !has_command && !has_parallel && !has_for_each {
            errors.push(format!(
                "{} must have at least one of: command, parallel, or for_each",
                step_label
            ));
        }

        // for_each steps must have either command or steps
        if has_for_each {
            let has_inner_steps = step.steps.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
            if !has_command && !has_inner_steps {
                errors.push(format!(
                    "{} has for_each but no command or steps",
                    step_label
                ));
            }
        }

        // retry.max_attempts must be >= 1
        if let Some(ref retry) = step.retry
            && retry.max_attempts < 1
        {
            errors.push(format!("{}: retry.max_attempts must be >= 1", step_label));
        }

        // timeout must be parseable as a duration
        if let Some(ref timeout) = step.timeout
            && parse_duration(timeout).is_err()
        {
            errors.push(format!(
                "{}: timeout '{}' is not a valid duration (use e.g. 5s, 30m, 2h)",
                step_label, timeout
            ));
        }

        // Warn if step has both if and unless
        if step.if_expr.is_some() && step.unless.is_some() {
            warnings.push(format!(
                "{} has both 'if' and 'unless' conditions; this may be confusing",
                step_label
            ));
        }

        // Recursively validate parallel sub-steps
        if let Some(ref parallel_steps) = step.parallel {
            validate_steps(parallel_steps, errors, warnings, &step_label);
        }

        // Recursively validate for_each inner steps
        if let Some(ref inner_steps) = step.steps {
            validate_steps(inner_steps, errors, warnings, &step_label);
        }

        // Recursively validate on_failure steps
        if let Some(ref failure_steps) = step.on_failure {
            validate_steps(
                failure_steps,
                errors,
                warnings,
                &format!("{} > on_failure", step_label),
            );
        }
    }
}

fn generate_sample(output: &PathBuf, output_format: OutputFormat) -> Result<()> {
    let sample_yaml = r#"name: "Model Processing Pipeline"
description: "Upload, translate, and download models with error handling"

defaults:
  retry:
    max_attempts: 3
    backoff: exponential
    delay: 5s
  timeout: 5m

variables:
  BUCKET: "my-models"

steps:
  - name: "Check if bucket exists"
    id: check_bucket
    command: "bucket info ${BUCKET}"
    ignore_failure: true

  - name: "Create bucket if missing"
    depends_on: ["Check if bucket exists"]
    command: "bucket create --key ${BUCKET} --policy persistent"
    if: "${{ steps.check_bucket.exit_code != 0 }}"

  - name: "Upload models in parallel"
    parallel:
      - name: "Upload building.rvt"
        command: "object upload ${BUCKET} building.rvt"
      - name: "Upload site.dwg"
        command: "object upload ${BUCKET} site.dwg"
    max_concurrency: 2

  - name: "Translate all models"
    for_each:
      var: MODEL
      in: ["building.rvt", "site.dwg"]
    steps:
      - name: "Start translation"
        command: "translate start urn:${BUCKET}/${MODEL}"
        retry:
          max_attempts: 2
          delay: 10s
      - name: "Wait for translation"
        command: "translate status urn:${BUCKET}/${MODEL} --wait"
        timeout: 60m

  - name: "Download results"
    for_each:
      var: MODEL
      in: ["building.rvt", "site.dwg"]
      parallel: true
      max_concurrency: 4
    command: "translate download urn:${BUCKET}/${MODEL} --out-dir ./output/${MODEL}"

  - name: "Cleanup bucket"
    command: "bucket delete ${BUCKET} -y"
    ignore_failure: true
"#;

    let content = if output.extension().map(|e| e == "json").unwrap_or(false) {
        let pipeline: Pipeline = serde_yaml::from_str(sample_yaml)
            .context("Failed to parse sample YAML (this is a bug)")?;
        serde_json::to_string_pretty(&pipeline)?
    } else {
        sample_yaml.to_string()
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
            #[derive(Serialize, schemars::JsonSchema)]
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

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(
            parse_duration("5s").unwrap(),
            std::time::Duration::from_secs(5)
        );
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(
            parse_duration("30m").unwrap(),
            std::time::Duration::from_secs(1800)
        );
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(
            parse_duration("2h").unwrap(),
            std::time::Duration::from_secs(7200)
        );
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn test_eval_expr_simple_eq() {
        let mut ctx = HashMap::new();
        ctx.insert("upload".to_string(), StepResult { exit_code: 0 });
        let ctx = Arc::new(Mutex::new(ctx));
        assert!(eval_expression("${{ steps.upload.exit_code == 0 }}", &ctx).unwrap());
    }

    #[test]
    fn test_eval_expr_not_eq() {
        let mut ctx = HashMap::new();
        ctx.insert("check".to_string(), StepResult { exit_code: 1 });
        let ctx = Arc::new(Mutex::new(ctx));
        assert!(eval_expression("${{ steps.check.exit_code != 0 }}", &ctx).unwrap());
    }

    #[test]
    fn test_eval_expr_no_template_fallback() {
        let ctx = Arc::new(Mutex::new(HashMap::new()));
        assert!(eval_expression("true", &ctx).unwrap());
        assert!(!eval_expression("false", &ctx).unwrap());
        assert!(!eval_expression("0", &ctx).unwrap());
    }

    #[test]
    fn test_eval_expr_missing_step() {
        let ctx = Arc::new(Mutex::new(HashMap::new()));
        assert!(eval_expression("${{ steps.missing.exit_code == 0 }}", &ctx).is_err());
    }

    #[test]
    fn test_parallel_step_deserialization() {
        let yaml = r#"
name: "Parallel Test"
steps:
  - name: Upload all
    parallel:
      - name: Upload A
        command: "object upload bucket file-a.rvt"
      - name: Upload B
        command: "object upload bucket file-b.rvt"
    max_concurrency: 2
"#;
        let pipeline: Pipeline = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(pipeline.steps.len(), 1);
        let step = &pipeline.steps[0];
        assert!(step.parallel.is_some());
        assert_eq!(step.parallel.as_ref().unwrap().len(), 2);
        assert_eq!(step.max_concurrency, Some(2));
    }

    #[test]
    fn test_for_each_deserialization() {
        let yaml = r#"
name: "ForEach Test"
steps:
  - name: Process each model
    for_each:
      var: model
      in: ["a.rvt", "b.rvt", "c.dwg"]
      parallel: true
      max_concurrency: 3
    command: "translate start ${model}"
"#;
        let pipeline: Pipeline = serde_yaml::from_str(yaml).unwrap();
        let step = &pipeline.steps[0];
        let fe = step.for_each.as_ref().unwrap();
        assert_eq!(fe.var, "model");
        assert_eq!(fe.items.len(), 3);
        assert!(fe.parallel);
        assert_eq!(fe.max_concurrency, Some(3));
    }

    #[test]
    fn test_v2_full_pipeline_deserialization() {
        let yaml = r#"
name: "Full V2 Test"
defaults:
  retry:
    max_attempts: 3
    backoff: exponential
    delay: 10s
  timeout: 5m
variables:
  bucket: "test-bucket"
steps:
  - name: Check
    id: check
    command: "bucket info ${bucket}"
    ignore_failure: true
  - name: Create
    command: "bucket create --key ${bucket}"
    if: "${{ steps.check.exit_code != 0 }}"
    retry:
      max_attempts: 2
      delay: 5s
    timeout: 30s
    on_failure:
      - name: Log error
        command: "api get /health"
  - name: Parallel uploads
    parallel:
      - name: Upload A
        command: "object upload ${bucket} a.rvt"
      - name: Upload B
        command: "object upload ${bucket} b.rvt"
    max_concurrency: 2
  - name: Process each
    for_each:
      var: file
      in: ["a.rvt", "b.rvt"]
      parallel: true
      max_concurrency: 3
    command: "translate start ${file}"
  - name: Cleanup
    command: "bucket delete ${bucket} -y"
    unless: "${{ steps.check.exit_code != 0 }}"
"#;
        let pipeline: Pipeline = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(pipeline.name, "Full V2 Test");
        assert!(pipeline.defaults.retry.is_some());
        let defaults_retry = pipeline.defaults.retry.as_ref().unwrap();
        assert_eq!(defaults_retry.max_attempts, 3);
        assert_eq!(defaults_retry.delay, "10s");
        assert!(matches!(
            defaults_retry.backoff,
            BackoffStrategy::Exponential
        ));
        assert_eq!(pipeline.defaults.timeout, Some("5m".to_string()));
        assert_eq!(pipeline.steps.len(), 5);

        // Check step
        assert_eq!(pipeline.steps[0].id, Some("check".to_string()));
        assert!(pipeline.steps[0].ignore_failure);

        // Create step
        assert_eq!(
            pipeline.steps[1].if_expr.as_deref(),
            Some("${{ steps.check.exit_code != 0 }}")
        );
        let step_retry = pipeline.steps[1].retry.as_ref().unwrap();
        assert_eq!(step_retry.max_attempts, 2);
        assert_eq!(step_retry.delay, "5s");
        let on_failure = pipeline.steps[1].on_failure.as_ref().unwrap();
        assert_eq!(on_failure.len(), 1);
        assert_eq!(on_failure[0].name, "Log error");

        // Parallel step
        assert!(pipeline.steps[2].parallel.is_some());
        assert_eq!(pipeline.steps[2].max_concurrency, Some(2));

        // ForEach step
        let fe = pipeline.steps[3].for_each.as_ref().unwrap();
        assert_eq!(fe.var, "file");
        assert_eq!(fe.items.len(), 2);
        assert!(fe.parallel);
        assert_eq!(fe.max_concurrency, Some(3));

        // Unless step
        assert_eq!(
            pipeline.steps[4].unless.as_deref(),
            Some("${{ steps.check.exit_code != 0 }}")
        );
    }

    #[test]
    fn test_v2_pipeline_json_roundtrip() {
        let yaml = r#"
name: "Roundtrip Test"
steps:
  - name: Test
    command: "auth test"
    retry:
      max_attempts: 2
      delay: 3s
"#;
        let pipeline: Pipeline = serde_yaml::from_str(yaml).unwrap();
        let json = serde_json::to_string_pretty(&pipeline).unwrap();
        let roundtrip: Pipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.name, "Roundtrip Test");
        assert_eq!(roundtrip.steps[0].name, "Test");
        assert_eq!(roundtrip.steps[0].command.as_deref(), Some("auth test"));
        let retry = roundtrip.steps[0].retry.as_ref().unwrap();
        assert_eq!(retry.max_attempts, 2);
        assert_eq!(retry.delay, "3s");
    }

    // ==================== Variable Substitution Tests ====================

    #[test]
    fn test_substitute_variables_basic() {
        let mut vars = HashMap::new();
        vars.insert("BUCKET".to_string(), "my-bucket".to_string());
        let result = substitute_variables("bucket info ${BUCKET}", &vars).unwrap();
        assert_eq!(result, "bucket info my-bucket");
    }

    #[test]
    fn test_substitute_variables_multiple() {
        let mut vars = HashMap::new();
        vars.insert("BUCKET".to_string(), "my-bucket".to_string());
        vars.insert("FILE".to_string(), "model.rvt".to_string());
        let result =
            substitute_variables("object upload ${BUCKET} ${FILE}", &vars).unwrap();
        assert_eq!(result, "object upload my-bucket model.rvt");
    }

    #[test]
    fn test_substitute_variables_no_vars() {
        let vars = HashMap::new();
        let result = substitute_variables("auth test", &vars).unwrap();
        assert_eq!(result, "auth test");
    }

    #[test]
    fn test_substitute_variables_shell_metachar_rejected() {
        let mut vars = HashMap::new();
        vars.insert("BAD".to_string(), "value; rm -rf /".to_string());
        let result = substitute_variables("echo ${BAD}", &vars);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("metacharacters")
        );
    }

    #[test]
    fn test_substitute_variables_pipe_rejected() {
        let mut vars = HashMap::new();
        vars.insert("CMD".to_string(), "ls | cat".to_string());
        assert!(substitute_variables("${CMD}", &vars).is_err());
    }

    #[test]
    fn test_substitute_variables_dollar_sign_in_value() {
        let mut vars = HashMap::new();
        vars.insert("VAR".to_string(), "has$dollar".to_string());
        assert!(substitute_variables("${VAR}", &vars).is_err());
    }

    // ==================== Validate Steps Tests ====================

    #[test]
    fn test_validate_steps_valid_command() {
        let steps = vec![Step {
            name: "Valid".to_string(),
            command: Some("bucket list".to_string()),
            ..Step::default()
        }];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        validate_steps(&steps, &mut errors, &mut warnings, "");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_steps_no_action() {
        let steps = vec![Step {
            name: "Empty".to_string(),
            ..Step::default()
        }];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        validate_steps(&steps, &mut errors, &mut warnings, "");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must have at least one of"));
    }

    #[test]
    fn test_validate_steps_for_each_without_command_or_steps() {
        let steps = vec![Step {
            name: "Bad ForEach".to_string(),
            for_each: Some(ForEachConfig {
                var: "x".to_string(),
                items: vec!["a".to_string()],
                parallel: false,
                max_concurrency: None,
            }),
            ..Step::default()
        }];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        validate_steps(&steps, &mut errors, &mut warnings, "");
        assert!(errors.iter().any(|e| e.contains("no command or steps")));
    }

    #[test]
    fn test_validate_steps_if_and_unless_warning() {
        let steps = vec![Step {
            name: "Both Conditions".to_string(),
            command: Some("auth test".to_string()),
            if_expr: Some("true".to_string()),
            unless: Some("false".to_string()),
            ..Step::default()
        }];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        validate_steps(&steps, &mut errors, &mut warnings, "");
        assert!(errors.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("both 'if' and 'unless'"));
    }

    #[test]
    fn test_validate_steps_invalid_retry() {
        let steps = vec![Step {
            name: "Bad Retry".to_string(),
            command: Some("auth test".to_string()),
            retry: Some(RetryConfig {
                max_attempts: 0,
                backoff: BackoffStrategy::Fixed,
                delay: "5s".to_string(),
                on: vec![],
            }),
            ..Step::default()
        }];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        validate_steps(&steps, &mut errors, &mut warnings, "");
        assert!(errors.iter().any(|e| e.contains("max_attempts must be >= 1")));
    }

    #[test]
    fn test_validate_steps_invalid_timeout() {
        let steps = vec![Step {
            name: "Bad Timeout".to_string(),
            command: Some("auth test".to_string()),
            timeout: Some("not-a-duration".to_string()),
            ..Step::default()
        }];
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        validate_steps(&steps, &mut errors, &mut warnings, "");
        assert!(errors.iter().any(|e| e.contains("not a valid duration")));
    }

    // ==================== Expression Evaluation Tests ====================

    #[test]
    fn test_eval_expr_and() {
        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), StepResult { exit_code: 0 });
        ctx.insert("b".to_string(), StepResult { exit_code: 0 });
        let ctx = Arc::new(Mutex::new(ctx));
        assert!(eval_expression(
            "${{ steps.a.exit_code == 0 && steps.b.exit_code == 0 }}",
            &ctx
        ).unwrap());
    }

    #[test]
    fn test_eval_expr_and_false() {
        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), StepResult { exit_code: 0 });
        ctx.insert("b".to_string(), StepResult { exit_code: 1 });
        let ctx = Arc::new(Mutex::new(ctx));
        assert!(!eval_expression(
            "${{ steps.a.exit_code == 0 && steps.b.exit_code == 0 }}",
            &ctx
        ).unwrap());
    }

    #[test]
    fn test_eval_expr_or() {
        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), StepResult { exit_code: 1 });
        ctx.insert("b".to_string(), StepResult { exit_code: 0 });
        let ctx = Arc::new(Mutex::new(ctx));
        assert!(eval_expression(
            "${{ steps.a.exit_code == 0 || steps.b.exit_code == 0 }}",
            &ctx
        ).unwrap());
    }

    #[test]
    fn test_eval_expr_negation() {
        let mut ctx = HashMap::new();
        ctx.insert("step1".to_string(), StepResult { exit_code: 1 });
        let ctx = Arc::new(Mutex::new(ctx));
        assert!(eval_expression(
            "${{ !steps.step1.exit_code == 0 }}",
            &ctx
        ).unwrap());
    }

    #[test]
    fn test_parse_duration_edge_cases() {
        assert_eq!(parse_duration("0s").unwrap(), std::time::Duration::from_secs(0));
        assert_eq!(parse_duration("1s").unwrap(), std::time::Duration::from_secs(1));
        assert_eq!(parse_duration("1m").unwrap(), std::time::Duration::from_secs(60));
        assert_eq!(parse_duration("1h").unwrap(), std::time::Duration::from_secs(3600));
        assert!(parse_duration("5d").is_err());
        assert!(parse_duration("  ").is_err());
    }

    #[test]
    fn test_eval_expr_operator_precedence_and_over_or() {
        let mut ctx = HashMap::new();
        ctx.insert("a".to_string(), StepResult { exit_code: 0 }); // true
        ctx.insert("b".to_string(), StepResult { exit_code: 1 }); // false
        ctx.insert("c".to_string(), StepResult { exit_code: 1 }); // false
        let ctx = Arc::new(Mutex::new(ctx));

        // Should evaluate as: true || (false && false) == true
        assert!(eval_expression(
            "${{ steps.a.exit_code == 0 || steps.b.exit_code == 0 && steps.c.exit_code == 0 }}",
            &ctx
        )
        .unwrap());
    }

    #[test]
    fn test_parse_var_assignment_valid() {
        let parsed = parse_var_assignment("BUCKET=my-bucket").unwrap();
        assert_eq!(parsed, ("BUCKET".to_string(), "my-bucket".to_string()));
    }

    #[test]
    fn test_parse_var_assignment_value_with_equals() {
        let parsed = parse_var_assignment("TOKEN=abc=def").unwrap();
        assert_eq!(parsed, ("TOKEN".to_string(), "abc=def".to_string()));
    }

    #[test]
    fn test_parse_var_assignment_invalid() {
        assert!(parse_var_assignment("missing_equals").is_err());
        assert!(parse_var_assignment("=value").is_err());
    }

    #[test]
    fn test_apply_variable_overrides_cli_wins() {
        let mut vars = HashMap::from([
            ("BUCKET".to_string(), "from-file".to_string()),
            ("REGION".to_string(), "US".to_string()),
        ]);
        let overrides = vec![
            ("BUCKET".to_string(), "from-cli".to_string()),
            ("MODEL".to_string(), "test.rvt".to_string()),
        ];

        apply_variable_overrides(&mut vars, &overrides);

        assert_eq!(vars.get("BUCKET"), Some(&"from-cli".to_string()));
        assert_eq!(vars.get("REGION"), Some(&"US".to_string()));
        assert_eq!(vars.get("MODEL"), Some(&"test.rvt".to_string()));
    }
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

/// Trigger type for a scheduled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
enum PipelineTrigger {
    Manual,
    Cron { expression: String },
    Webhook { event: String },
}

/// A scheduled pipeline definition written to `.pipeline.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ScheduledPipeline {
    name: String,
    trigger: PipelineTrigger,
    #[serde(default)]
    source: Option<String>,
    action: String,
    #[serde(default)]
    notify: bool,
    #[serde(default)]
    serverless: bool,
    #[serde(default)]
    steps: Vec<Step>,
}

#[allow(clippy::too_many_arguments)]
fn create_pipeline(
    name: &str,
    source: Option<String>,
    cron: Option<String>,
    action: &str,
    notify: bool,
    serverless: bool,
    out_file: &PathBuf,
    output_format: OutputFormat,
) -> Result<()> {
    let trigger = if let Some(ref expr) = cron {
        PipelineTrigger::Cron {
            expression: expr.clone(),
        }
    } else {
        PipelineTrigger::Manual
    };

    // Build a default step based on the action
    let step_command = match action {
        "translate" => {
            let mut cmd = "translate start".to_string();
            if let Some(ref s) = source {
                cmd.push_str(&format!(" {}", s));
            }
            if serverless {
                cmd.push_str(" --serverless");
            }
            cmd.push_str(" --wait");
            cmd
        }
        "upload" => {
            let mut cmd = "object upload".to_string();
            if let Some(ref s) = source {
                cmd.push_str(&format!(" {}", s));
            }
            cmd
        }
        "extract-props" => {
            let mut cmd = "translate properties".to_string();
            if let Some(ref s) = source {
                cmd.push_str(&format!(" {}", s));
            }
            cmd
        }
        other => other.to_string(),
    };

    let step = Step {
        name: format!("{} step", action),
        command: Some(step_command),
        ..Step::default()
    };

    let pipeline = ScheduledPipeline {
        name: name.to_string(),
        trigger,
        source: source.clone(),
        action: action.to_string(),
        notify,
        serverless,
        steps: vec![step],
    };

    let yaml = serde_yaml::to_string(&pipeline).context("Failed to serialize pipeline")?;
    std::fs::write(out_file, &yaml)
        .with_context(|| format!("Failed to write {}", out_file.display()))?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Pipeline created: {}", "✓".green(), out_file.display());
            println!("  Name:       {}", name);
            println!("  Action:     {}", action);
            if let Some(ref expr) = cron {
                println!("  Cron:       {}", expr);
            } else {
                println!("  Trigger:    manual");
            }
            if serverless {
                println!("  Dispatch:   serverless (Fly.io)");
            }
            if notify {
                println!("  Notify:     enabled");
            }
        }
        _ => {
            output_format.write(&pipeline)?;
        }
    }

    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
// Pipeline diff
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct StepDiffEntry {
    name: String,
    change: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Vec<String>>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct PipelineDiffOutput {
    file1: String,
    file2: String,
    added: Vec<StepDiffEntry>,
    removed: Vec<StepDiffEntry>,
    changed: Vec<StepDiffEntry>,
    reordered: bool,
    identical: bool,
}

async fn diff_pipelines(file1: &PathBuf, file2: &PathBuf, output_format: OutputFormat) -> Result<()> {
    let p1 = load_pipeline(file1).await?;
    let p2 = load_pipeline(file2).await?;

    let names1: Vec<&str> = p1.steps.iter().map(|s| s.name.as_str()).collect();
    let names2: Vec<&str> = p2.steps.iter().map(|s| s.name.as_str()).collect();

    let set1: std::collections::HashSet<&str> = names1.iter().copied().collect();
    let set2: std::collections::HashSet<&str> = names2.iter().copied().collect();

    let mut added: Vec<StepDiffEntry> = set2
        .difference(&set1)
        .map(|n| StepDiffEntry {
            name: n.to_string(),
            change: "added".to_string(),
            details: None,
        })
        .collect();
    added.sort_by(|a, b| a.name.cmp(&b.name));

    let mut removed: Vec<StepDiffEntry> = set1
        .difference(&set2)
        .map(|n| StepDiffEntry {
            name: n.to_string(),
            change: "removed".to_string(),
            details: None,
        })
        .collect();
    removed.sort_by(|a, b| a.name.cmp(&b.name));

    let mut changed: Vec<StepDiffEntry> = Vec::new();
    let map1: HashMap<&str, &Step> = p1.steps.iter().map(|s| (s.name.as_str(), s)).collect();
    let map2: HashMap<&str, &Step> = p2.steps.iter().map(|s| (s.name.as_str(), s)).collect();

    for name in set1.intersection(&set2) {
        let s1 = map1[name];
        let s2 = map2[name];
        let mut diffs = Vec::new();
        if s1.command != s2.command {
            diffs.push(format!(
                "command: {:?} -> {:?}",
                s1.command.as_deref().unwrap_or(""),
                s2.command.as_deref().unwrap_or("")
            ));
        }
        if s1.depends_on != s2.depends_on {
            diffs.push(format!(
                "depends_on: {:?} -> {:?}",
                s1.depends_on,
                s2.depends_on
            ));
        }
        if s1.if_expr != s2.if_expr {
            diffs.push(format!(
                "condition: {:?} -> {:?}",
                s1.if_expr.as_deref().unwrap_or(""),
                s2.if_expr.as_deref().unwrap_or("")
            ));
        }
        if !diffs.is_empty() {
            changed.push(StepDiffEntry {
                name: name.to_string(),
                change: "changed".to_string(),
                details: Some(diffs),
            });
        }
    }
    changed.sort_by(|a, b| a.name.cmp(&b.name));

    // Check for reordering among common steps
    let common_order1: Vec<&str> = names1.iter().copied().filter(|n| set2.contains(n)).collect();
    let common_order2: Vec<&str> = names2.iter().copied().filter(|n| set1.contains(n)).collect();
    let reordered = common_order1 != common_order2;

    let identical = added.is_empty() && removed.is_empty() && changed.is_empty() && !reordered;

    let diff = PipelineDiffOutput {
        file1: file1.display().to_string(),
        file2: file2.display().to_string(),
        added,
        removed,
        changed,
        reordered,
        identical,
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "\n{} {} vs {}",
                "Pipeline Diff:".bold(),
                file1.display().to_string().cyan(),
                file2.display().to_string().cyan()
            );
            println!("{}", "─".repeat(70));

            if diff.identical {
                println!("{} Pipelines are identical.", "\u{2713}".green().bold());
                return Ok(());
            }

            for entry in &diff.added {
                println!(
                    "  {} {} {}",
                    "+".green().bold(),
                    entry.name.green(),
                    "(added)".dimmed()
                );
            }
            for entry in &diff.removed {
                println!(
                    "  {} {} {}",
                    "-".red().bold(),
                    entry.name.red(),
                    "(removed)".dimmed()
                );
            }
            for entry in &diff.changed {
                println!(
                    "  {} {} {}",
                    "~".yellow().bold(),
                    entry.name.yellow(),
                    "(changed)".dimmed()
                );
                if let Some(ref details) = entry.details {
                    for d in details {
                        println!("      {} {}", "↳".dimmed(), d.dimmed());
                    }
                }
            }
            if diff.reordered {
                println!(
                    "  {} {}",
                    "⇄".yellow().bold(),
                    "Steps have been reordered".yellow()
                );
            }

            println!("{}", "─".repeat(70));
            println!(
                "  {} added, {} removed, {} changed{}",
                diff.added.len().to_string().green(),
                diff.removed.len().to_string().red(),
                diff.changed.len().to_string().yellow(),
                if diff.reordered { ", reordered" } else { "" }
            );
        }
        _ => {
            output_format.write(&diff)?;
        }
    }

    Ok(())
}
