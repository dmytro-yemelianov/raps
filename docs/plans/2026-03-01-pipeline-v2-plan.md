# Pipeline v2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current sequential pipeline engine with a v2 engine supporting retries, timeouts, conditionals, parallel steps, and for_each loops.

**Architecture:** Extend `raps-cli/src/commands/pipeline.rs` (currently 495 lines). New structs for retry/timeout/parallel/for_each config parsed via serde. Execution engine refactored into an async executor that handles parallel spawning via `tokio::spawn` + `Semaphore` (same pattern as batch upload). Expression evaluation for `${{ steps.id.exit_code }}` via a simple recursive-descent parser.

**Tech Stack:** Rust, tokio (async), serde (YAML/JSON), shlex (command parsing), anyhow (errors), indicatif (progress), Arc/Semaphore (concurrency)

---

### Task 1: Restructure Pipeline Data Model

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs:49-76`

**Step 1: Replace Pipeline and PipelineStep structs**

Replace the existing structs (lines 49-76) with the v2 data model:

```rust
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
    // A step is ONE of: command, parallel, or for_each
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub parallel: Option<Vec<Step>>,
    #[serde(default)]
    pub for_each: Option<ForEachConfig>,
    // Nested steps for for_each
    #[serde(default)]
    pub steps: Option<Vec<Step>>,
    // Control flow
    #[serde(default, rename = "if")]
    pub if_expr: Option<String>,
    #[serde(default)]
    pub unless: Option<String>,
    #[serde(default)]
    pub ignore_failure: bool,
    // Error handling
    #[serde(default)]
    pub retry: Option<RetryConfig>,
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub on_failure: Option<Vec<Step>>,
    // Parallel config
    #[serde(default)]
    pub max_concurrency: Option<usize>,
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
```

**Step 2: Add a StepContext struct to track step results**

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct StepResult {
    pub exit_code: i32,
}

type StepContext = Arc<Mutex<HashMap<String, StepResult>>>;
```

**Step 3: Update the existing tests to use the new structs**

Update `test_pipeline_deserialization_yaml` and `test_pipeline_deserialization_json` at lines 429-493 to use new field names (`ignore_failure` instead of `continue_on_error`, `if_expr` instead of `condition`).

**Step 4: Run tests**

Run: `cargo test -p raps-cli pipeline`
Expected: All existing tests pass with updated field names.

**Step 5: Commit**

```
feat(pipeline): restructure data model for v2
```

---

### Task 2: Duration Parser & Retry Engine

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs`

**Step 1: Write failing tests for duration parsing**

```rust
#[test]
fn test_parse_duration_seconds() {
    assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
}

#[test]
fn test_parse_duration_minutes() {
    assert_eq!(parse_duration("30m").unwrap(), Duration::from_secs(1800));
}

#[test]
fn test_parse_duration_hours() {
    assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
}

#[test]
fn test_parse_duration_invalid() {
    assert!(parse_duration("abc").is_err());
    assert!(parse_duration("").is_err());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p raps-cli parse_duration`
Expected: FAIL — function not defined.

**Step 3: Implement parse_duration**

```rust
fn parse_duration(s: &str) -> Result<std::time::Duration> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty duration string");
    }
    let (num_str, suffix) = if s.ends_with('s') {
        (&s[..s.len() - 1], 's')
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], 'm')
    } else if s.ends_with('h') {
        (&s[..s.len() - 1], 'h')
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
```

**Step 4: Write failing tests for retry logic**

```rust
#[tokio::test]
async fn test_retry_succeeds_on_first_try() {
    let config = RetryConfig {
        max_attempts: 3,
        backoff: BackoffStrategy::Fixed,
        delay: "1s".to_string(),
        on: vec![],
    };
    let call_count = Arc::new(Mutex::new(0));
    let cc = call_count.clone();
    let result = execute_with_retry(&config, || {
        *cc.lock().unwrap() += 1;
        Ok(0)
    })
    .await;
    assert_eq!(result.unwrap(), 0);
    assert_eq!(*call_count.lock().unwrap(), 1);
}

#[tokio::test]
async fn test_retry_succeeds_on_third_try() {
    let config = RetryConfig {
        max_attempts: 3,
        backoff: BackoffStrategy::Fixed,
        delay: "0s".to_string(),
        on: vec![],
    };
    let call_count = Arc::new(Mutex::new(0));
    let cc = call_count.clone();
    let result = execute_with_retry(&config, || {
        let mut count = cc.lock().unwrap();
        *count += 1;
        if *count < 3 { Ok(1) } else { Ok(0) }
    })
    .await;
    assert_eq!(result.unwrap(), 0);
    assert_eq!(*call_count.lock().unwrap(), 3);
}

#[tokio::test]
async fn test_retry_exhausted() {
    let config = RetryConfig {
        max_attempts: 2,
        backoff: BackoffStrategy::Fixed,
        delay: "0s".to_string(),
        on: vec![],
    };
    let result = execute_with_retry(&config, || Ok(1)).await;
    assert_eq!(result.unwrap(), 1); // last exit code
}
```

**Step 5: Implement execute_with_retry**

```rust
async fn execute_with_retry<F>(config: &RetryConfig, mut run: F) -> Result<i32>
where
    F: FnMut() -> Result<i32>,
{
    let base_delay = parse_duration(&config.delay)?;
    let mut last_exit = -1;

    for attempt in 1..=config.max_attempts {
        match run() {
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
```

**Step 6: Run tests**

Run: `cargo test -p raps-cli retry`
Expected: PASS

**Step 7: Commit**

```
feat(pipeline): add duration parser and retry engine
```

---

### Task 3: Expression Evaluator for Conditionals

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs`

**Step 1: Write failing tests**

```rust
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
    // Non-template strings use old truthiness logic
    assert!(eval_expression("true", &ctx).unwrap());
    assert!(!eval_expression("false", &ctx).unwrap());
    assert!(!eval_expression("0", &ctx).unwrap());
}

#[test]
fn test_eval_expr_missing_step() {
    let ctx = Arc::new(Mutex::new(HashMap::new()));
    assert!(eval_expression("${{ steps.missing.exit_code == 0 }}", &ctx).is_err());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p raps-cli eval_expr`
Expected: FAIL

**Step 3: Implement eval_expression**

```rust
fn eval_expression(expr: &str, context: &StepContext) -> Result<bool> {
    let trimmed = expr.trim();

    // Check for ${{ ... }} template syntax
    if let Some(inner) = trimmed.strip_prefix("${{").and_then(|s| s.strip_suffix("}}")) {
        let inner = inner.trim();
        return eval_comparison(inner, context);
    }

    // Fallback: simple truthiness (backward compat with condition field)
    let lower = trimmed.to_lowercase();
    Ok(!matches!(lower.as_str(), "false" | "0" | ""))
}

fn eval_comparison(expr: &str, context: &StepContext) -> Result<bool> {
    // Support: steps.<id>.exit_code == N, steps.<id>.exit_code != N
    // Support: <left> && <right>, <left> || <right>
    if let Some((left, right)) = expr.split_once("&&") {
        return Ok(eval_comparison(left.trim(), context)?
            && eval_comparison(right.trim(), context)?);
    }
    if let Some((left, right)) = expr.split_once("||") {
        return Ok(eval_comparison(left.trim(), context)?
            || eval_comparison(right.trim(), context)?);
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
```

**Step 4: Run tests**

Run: `cargo test -p raps-cli eval_expr`
Expected: PASS

**Step 5: Commit**

```
feat(pipeline): add expression evaluator for conditionals
```

---

### Task 4: Rewrite Execution Engine

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs` (replace `run_pipeline` function, lines 124-263)

**Step 1: Write the new execute_step function**

This is the core recursive executor that handles all step types:

```rust
async fn execute_step(
    step: &Step,
    variables: &HashMap<String, String>,
    context: &StepContext,
    defaults: &PipelineDefaults,
    output_format: &OutputFormat,
    dry_run: bool,
) -> Result<StepOutcome> {
    // Evaluate if/unless conditions
    if let Some(ref expr) = step.if_expr {
        if !eval_expression(expr, context)? {
            return Ok(StepOutcome::Skipped);
        }
    }
    if let Some(ref expr) = step.unless {
        if eval_expression(expr, context)? {
            return Ok(StepOutcome::Skipped);
        }
    }

    let outcome = if let Some(ref cmd) = step.command {
        execute_command_step(step, cmd, variables, defaults, dry_run).await?
    } else if let Some(ref parallel_steps) = step.parallel {
        execute_parallel_steps(
            parallel_steps,
            variables,
            context,
            defaults,
            output_format,
            dry_run,
            step.max_concurrency.unwrap_or(10),
        )
        .await?
    } else if let Some(ref for_each) = step.for_each {
        let inner_steps = step.steps.as_deref().unwrap_or(&[]);
        execute_for_each(
            for_each,
            step,
            inner_steps,
            variables,
            context,
            defaults,
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
    if matches!(outcome, StepOutcome::Failed(_)) {
        if let Some(ref failure_steps) = step.on_failure {
            for fs in failure_steps {
                let _ = execute_step(fs, variables, context, defaults, output_format, dry_run).await;
            }
        }
    }

    Ok(outcome)
}

#[derive(Debug)]
enum StepOutcome {
    Success(i32),
    Failed(i32),
    Skipped,
}
```

**Step 2: Write execute_command_step with retry and timeout**

```rust
async fn execute_command_step(
    step: &Step,
    cmd: &str,
    variables: &HashMap<String, String>,
    defaults: &PipelineDefaults,
    dry_run: bool,
) -> Result<StepOutcome> {
    // Substitute variables
    let mut command = cmd.to_string();
    for (key, value) in variables {
        const SHELL_META: &[char] = &['|', '&', ';', '$', '`', '(', ')', '{', '}', '<', '>'];
        if value.contains(SHELL_META) {
            anyhow::bail!("Pipeline variable '{}' contains shell metacharacters", key);
        }
        command = command.replace(&format!("${{{}}}", key), value);
        command = command.replace(&format!("${}", key), value);
    }

    if dry_run {
        println!("  {} Would execute: raps {}", "→".dimmed(), command);
        return Ok(StepOutcome::Success(0));
    }

    // Determine retry config (step overrides defaults)
    let retry = step
        .retry
        .as_ref()
        .or(defaults.retry.as_ref())
        .cloned()
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

    let cmd_clone = command.clone();
    let run_fn = move || execute_raps_command(&cmd_clone);

    let result = if let Some(timeout_dur) = timeout_duration {
        match tokio::time::timeout(timeout_dur, execute_with_retry(&retry, run_fn)).await {
            Ok(r) => r?,
            Err(_) => {
                eprintln!("  Step timed out after {}s", timeout_dur.as_secs());
                return Ok(StepOutcome::Failed(-1));
            }
        }
    } else {
        execute_with_retry(&retry, run_fn).await?
    };

    if result == 0 {
        Ok(StepOutcome::Success(0))
    } else {
        Ok(StepOutcome::Failed(result))
    }
}
```

Note: `execute_with_retry` needs adjustment — the closure `run` must be `FnMut` and called multiple times. Since `execute_raps_command` takes a `&str`, we need to clone the command string. Adjust the retry function signature to accept `&str` directly:

```rust
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
```

**Step 3: Rewrite run_pipeline to use execute_step**

```rust
async fn run_pipeline(
    file: &std::path::Path,
    global_ignore_failure: bool,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let pipeline = load_pipeline(file)?;
    let context: StepContext = Arc::new(Mutex::new(HashMap::new()));

    if output_format.supports_colors() {
        println!(
            "\n{} Running pipeline: {}",
            "▶".cyan().bold(),
            pipeline.name.bold()
        );
        if let Some(ref desc) = pipeline.description {
            println!("  {}", desc.dimmed());
        }
        println!();
    }

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;

    for (i, step) in pipeline.steps.iter().enumerate() {
        if output_format.supports_colors() {
            println!(
                "Step {}/{}: {}",
                (i + 1).to_string().cyan(),
                pipeline.steps.len().to_string().cyan(),
                step.name.bold()
            );
        }

        let outcome = execute_step(
            step,
            &pipeline.variables,
            &context,
            &pipeline.defaults,
            &output_format,
            dry_run,
        )
        .await?;

        match outcome {
            StepOutcome::Success(_) => {
                if output_format.supports_colors() {
                    println!("  {} Success", "✓".green().bold());
                }
                passed += 1;
            }
            StepOutcome::Failed(code) => {
                if output_format.supports_colors() {
                    println!("  {} Failed (exit code: {})", "✗".red().bold(), code);
                }
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
                skipped += 1;
            }
        }
    }

    // Summary
    if output_format.supports_colors() {
        println!("\n{}", "Pipeline Summary:".bold());
        println!(
            "  {} passed, {} failed, {} skipped",
            passed.to_string().green(),
            failed.to_string().red(),
            skipped.to_string().yellow()
        );
    }

    if !output_format.supports_colors() {
        let result = serde_json::json!({
            "name": pipeline.name,
            "passed": passed,
            "failed": failed,
            "skipped": skipped,
            "success": failed == 0,
        });
        match output_format {
            OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&result)?),
            OutputFormat::Yaml => println!("{}", serde_yaml::to_string(&result)?),
            _ => println!("passed={} failed={} skipped={}", passed, failed, skipped),
        }
    }

    if failed > 0 && !global_ignore_failure {
        anyhow::bail!("Pipeline completed with {} failed step(s)", failed);
    }

    Ok(())
}
```

**Step 4: Run tests**

Run: `cargo test -p raps-cli pipeline`
Expected: PASS

**Step 5: Commit**

```
feat(pipeline): rewrite execution engine with retry, timeout, conditionals
```

---

### Task 5: Parallel Step Execution

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs`

**Step 1: Write failing test**

```rust
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
```

**Step 2: Run test to verify it passes (deserialization only)**

Run: `cargo test -p raps-cli test_parallel_step`
Expected: PASS (struct already supports it from Task 1)

**Step 3: Implement execute_parallel_steps**

```rust
async fn execute_parallel_steps(
    steps: &[Step],
    variables: &HashMap<String, String>,
    context: &StepContext,
    defaults: &PipelineDefaults,
    output_format: &OutputFormat,
    dry_run: bool,
    max_concurrency: usize,
) -> Result<StepOutcome> {
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut handles = Vec::new();

    for step in steps {
        let sem = semaphore.clone();
        let step = step.clone();
        let vars = variables.clone();
        let ctx = context.clone();
        let defs = defaults.clone();
        let fmt = output_format.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            execute_step(&step, &vars, &ctx, &defs, &fmt, dry_run).await
        });
        handles.push(handle);
    }

    let results = futures_util::future::join_all(handles).await;
    let mut any_failed = false;
    let mut last_code = 0;

    for result in results {
        match result {
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
```

**Step 4: Ensure futures-util is in dependencies**

Check `raps-cli/Cargo.toml` for `futures-util`. If missing, add to workspace deps.

**Step 5: Run tests**

Run: `cargo test -p raps-cli pipeline`
Expected: PASS

**Step 6: Commit**

```
feat(pipeline): add parallel step execution with semaphore
```

---

### Task 6: For-Each Loop Execution

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs`

**Step 1: Write failing test**

```rust
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
```

**Step 2: Implement execute_for_each**

```rust
async fn execute_for_each(
    config: &ForEachConfig,
    parent_step: &Step,
    inner_steps: &[Step],
    variables: &HashMap<String, String>,
    context: &StepContext,
    defaults: &PipelineDefaults,
    output_format: &OutputFormat,
    dry_run: bool,
) -> Result<StepOutcome> {
    let max_concurrency = config.max_concurrency.unwrap_or(5);

    if config.parallel {
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut handles = Vec::new();

        for item in &config.items {
            let sem = semaphore.clone();
            let mut iter_vars = variables.clone();
            iter_vars.insert(config.var.clone(), item.clone());
            let ctx = context.clone();
            let defs = defaults.clone();
            let fmt = output_format.clone();

            // If there's a direct command, use it; otherwise use inner steps
            let steps_to_run: Vec<Step> = if let Some(ref cmd) = parent_step.command {
                vec![Step {
                    name: format!("{} [{}={}]", parent_step.name, config.var, item),
                    command: Some(cmd.clone()),
                    ..Step::default()
                }]
            } else {
                inner_steps.to_vec()
            };

            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                let mut any_failed = false;
                for step in &steps_to_run {
                    match execute_step(step, &iter_vars, &ctx, &defs, &fmt, dry_run).await? {
                        StepOutcome::Failed(c) => {
                            any_failed = true;
                            if !step.ignore_failure {
                                return Ok(StepOutcome::Failed(c));
                            }
                        }
                        _ => {}
                    }
                }
                Ok::<_, anyhow::Error>(if any_failed {
                    StepOutcome::Failed(1)
                } else {
                    StepOutcome::Success(0)
                })
            });
            handles.push(handle);
        }

        let results = futures_util::future::join_all(handles).await;
        for result in results {
            match result {
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
                inner_steps.to_vec()
            };

            for step in &steps_to_run {
                match execute_step(step, &iter_vars, context, defaults, output_format, dry_run)
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
```

Also add a Default impl for Step:

```rust
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
```

**Step 3: Run tests**

Run: `cargo test -p raps-cli pipeline`
Expected: PASS

**Step 4: Commit**

```
feat(pipeline): add for_each loop execution (sequential and parallel)
```

---

### Task 7: Update Sample Pipeline & Validation

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs` (generate_sample and validate_pipeline functions)

**Step 1: Update generate_sample to showcase v2 features**

Replace the sample pipeline template to demonstrate retries, conditionals, parallel, and for_each:

```rust
fn generate_sample(output: &std::path::Path, output_format: OutputFormat) -> Result<()> {
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
    command: "bucket create --key ${BUCKET} --policy persistent"
    if: "${{ steps.check_bucket.exit_code != 0 }}"

  - name: "Upload models in parallel"
    parallel:
      - name: "Upload building.rvt"
        command: "object upload ${BUCKET} building.rvt"
        id: upload_building
      - name: "Upload site.dwg"
        command: "object upload ${BUCKET} site.dwg"
        id: upload_site
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
    command: "translate download urn:${BUCKET}/${MODEL} --output ./output/${MODEL}"

  - name: "Cleanup bucket"
    command: "bucket delete ${BUCKET} -y"
    ignore_failure: true
    on_failure:
      - name: "Log cleanup failure"
        command: "api get /oss/v2/buckets/${BUCKET}"
"#;

    // Write YAML directly (it's the canonical format)
    let content = if matches!(output_format, OutputFormat::Json) {
        let pipeline: Pipeline = serde_yaml::from_str(sample_yaml)?;
        serde_json::to_string_pretty(&pipeline)?
    } else {
        sample_yaml.to_string()
    };

    std::fs::write(output, &content)
        .with_context(|| format!("Failed to write sample pipeline to {}", output.display()))?;

    println!("Sample pipeline written to {}", output.display());
    Ok(())
}
```

**Step 2: Update validate_pipeline for v2 fields**

Add validation for:
- Steps must have exactly one of: `command`, `parallel`, `for_each`
- `for_each` steps must have either `command` or `steps`
- `retry.max_attempts` must be >= 1
- `timeout` must be parseable as a duration
- `if`/`unless` expressions must use valid `${{ }}` syntax

**Step 3: Update the pipeline subcommand help text**

Update the `PipelineCommands` doc comments and the `--continue-on-error` flag name to `--ignore-failure`.

**Step 4: Run tests**

Run: `cargo test -p raps-cli pipeline`
Expected: PASS

**Step 5: Run full workspace check**

Run: `cargo check --workspace`
Expected: No errors

**Step 6: Commit**

```
feat(pipeline): update sample pipeline and validation for v2
```

---

### Task 8: Integration Tests

**Files:**
- Modify: `raps-cli/src/commands/pipeline.rs` (test module)

**Step 1: Add comprehensive YAML round-trip test**

```rust
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
    assert_eq!(pipeline.defaults.timeout, Some("5m".to_string()));
    assert_eq!(pipeline.steps.len(), 5);

    // Check step
    assert_eq!(pipeline.steps[0].id, Some("check".to_string()));
    assert!(pipeline.steps[0].ignore_failure);

    // Create step
    assert!(pipeline.steps[1].if_expr.is_some());
    assert!(pipeline.steps[1].retry.is_some());
    assert!(pipeline.steps[1].on_failure.is_some());

    // Parallel step
    assert!(pipeline.steps[2].parallel.is_some());
    assert_eq!(pipeline.steps[2].max_concurrency, Some(2));

    // ForEach step
    let fe = pipeline.steps[3].for_each.as_ref().unwrap();
    assert_eq!(fe.var, "file");
    assert_eq!(fe.items.len(), 2);
    assert!(fe.parallel);

    // Unless step
    assert!(pipeline.steps[4].unless.is_some());
}
```

**Step 2: Add JSON round-trip test**

```rust
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
    assert!(roundtrip.steps[0].retry.is_some());
}
```

**Step 3: Run all tests**

Run: `cargo test -p raps-cli pipeline`
Expected: PASS

**Step 4: Commit**

```
test(pipeline): add comprehensive v2 integration tests
```

---

### Task 9: Update Documentation

**Files:**
- Modify: `docs/features.md` (pipeline section)

**Step 1: Update the pipeline section in features.md**

Add v2 feature descriptions: retry, timeout, conditionals, parallel, for_each. Include a short YAML example showing the new features.

**Step 2: Commit**

```
docs: update pipeline documentation for v2 features
```

---

### Task 10: Final Verification

**Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues

**Step 4: Create a sample v2 pipeline file and dry-run it**

Create `test-pipeline.raps.yaml` with the v2 sample content and run:
```
cargo run -- pipeline run test-pipeline.raps.yaml --dry-run
```
Expected: All steps show "Would execute" without errors.

**Step 5: Clean up test file and commit**

```
chore: final pipeline v2 verification
```
