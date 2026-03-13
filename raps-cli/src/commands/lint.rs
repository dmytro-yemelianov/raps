// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps lint [PATH...]` — pre-commit validation command.
//!
//! Checks:
//! - `*.yaml`/`*.yml` pipeline files: syntax, `depends_on` references, cycles
//! - Any text file: secret scanning (reuses `object::secret_scan`)
//! - `.raps-project`: hub/project IDs non-empty, profile exists in config
//!
//! Exits with code 1 if any errors are found.

use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::commands::object::secret_scan;
use crate::commands::pipeline::{Pipeline, Step};
use crate::output::OutputFormat;

// ── Diagnostic types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Diagnostic {
    pub file: String,
    pub line: Option<usize>,
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LintReport {
    pub files_checked: usize,
    pub warnings: usize,
    pub errors: usize,
    pub diagnostics: Vec<Diagnostic>,
}

// ── Top-level entry point ─────────────────────────────────────────────────────

pub async fn run_lint(paths: Vec<PathBuf>, output_format: OutputFormat) -> Result<()> {
    // Collect files to lint
    let files = if paths.is_empty() {
        collect_files(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))?
    } else {
        let mut all = Vec::new();
        for p in &paths {
            if p.is_dir() {
                all.extend(collect_files(p)?);
            } else {
                all.push(p.clone());
            }
        }
        all
    };

    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    for file in &files {
        lint_file(file, &mut diagnostics);
    }

    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();

    let report = LintReport {
        files_checked: files.len(),
        warnings,
        errors,
        diagnostics,
    };

    match output_format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Csv => {
            print_csv(&report)?;
        }
        _ => {
            print_table(&report);
        }
    }

    if errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// ── File collection ───────────────────────────────────────────────────────────

fn collect_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_recursive(dir, &mut files)?;
    Ok(files)
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let rd = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip hidden directories and common build/deps dirs
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            collect_recursive(&path, out)?;
        } else if should_lint(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn should_lint(p: &Path) -> bool {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    name == ".raps-project" || ext == "yaml" || ext == "yml" || is_text_candidate(p)
}

/// Heuristic: lint text-like files for secrets (skip known binary extensions)
fn is_text_candidate(p: &Path) -> bool {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    !matches!(
        ext.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "ico"
            | "pdf"
            | "zip"
            | "tar"
            | "gz"
            | "tgz"
            | "rvt"
            | "dwg"
            | "nwd"
            | "nwc"
            | "ifc"
            | "fbx"
            | "obj"
            | "bin"
            | "exe"
            | "dll"
            | "so"
            | "a"
            | "lib"
            | "pdb"
            | "mp4"
            | "mp3"
            | "mov"
            | "avi"
            | "wav"
            | "flac"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
    )
}

// ── Per-file linting ──────────────────────────────────────────────────────────

fn lint_file(path: &Path, diags: &mut Vec<Diagnostic>) {
    let path_str = path.display().to_string();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // 1. Pipeline YAML lint
    if ext == "yaml" || ext == "yml" {
        lint_pipeline_yaml(path, &path_str, diags);
    }

    // 2. .raps-project lint
    if name == ".raps-project" {
        lint_raps_project(path, &path_str, diags);
    }

    // 3. Secret scan (all text files)
    if is_text_candidate(path) {
        lint_secrets(path, &path_str, diags);
    }
}

// ── Pipeline YAML linting ─────────────────────────────────────────────────────

fn lint_pipeline_yaml(path: &Path, path_str: &str, diags: &mut Vec<Diagnostic>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic {
                file: path_str.to_string(),
                line: None,
                severity: Severity::Error,
                code: "L001".to_string(),
                message: format!("Cannot read file: {}", e),
            });
            return;
        }
    };

    // Check if it looks like a raps pipeline (must have a `steps:` key)
    if !content.contains("steps:") {
        // Not a pipeline file — skip pipeline-specific checks
        return;
    }

    // Syntax check
    let pipeline: Pipeline = match serde_yaml::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            let line = e.location().map(|l| l.line());
            diags.push(Diagnostic {
                file: path_str.to_string(),
                line,
                severity: Severity::Error,
                code: "L002".to_string(),
                message: format!("Pipeline YAML syntax error: {}", e),
            });
            return;
        }
    };

    // Collect all step IDs/names (for depends_on validation)
    let mut all_ids: HashSet<String> = HashSet::new();
    collect_step_ids(&pipeline.steps, &mut all_ids);

    // Validate depends_on references and detect cycles
    let mut dep_graph: HashMap<String, Vec<String>> = HashMap::new();
    validate_depends_on(&pipeline.steps, &all_ids, path_str, diags, &mut dep_graph);

    // Cycle detection via Kahn's algorithm
    if let Some(cycle) = detect_cycle(&dep_graph) {
        diags.push(Diagnostic {
            file: path_str.to_string(),
            line: None,
            severity: Severity::Error,
            code: "L005".to_string(),
            message: format!("Dependency cycle detected: {}", cycle.join(" -> ")),
        });
    }

    // Warn on steps with no command and no parallel/sub-steps
    warn_empty_steps(&pipeline.steps, path_str, diags);
}

fn collect_step_ids(steps: &[Step], out: &mut HashSet<String>) {
    for step in steps {
        out.insert(step.name.clone());
        if let Some(id) = &step.id {
            out.insert(id.clone());
        }
        if let Some(sub) = &step.parallel {
            collect_step_ids(sub, out);
        }
        if let Some(sub) = &step.steps {
            collect_step_ids(sub, out);
        }
    }
}

fn validate_depends_on(
    steps: &[Step],
    all_ids: &HashSet<String>,
    path_str: &str,
    diags: &mut Vec<Diagnostic>,
    graph: &mut HashMap<String, Vec<String>>,
) {
    for step in steps {
        let node = step.id.clone().unwrap_or_else(|| step.name.clone());
        let deps = graph.entry(node.clone()).or_default();
        for dep in &step.depends_on {
            if !all_ids.contains(dep) {
                diags.push(Diagnostic {
                    file: path_str.to_string(),
                    line: None,
                    severity: Severity::Error,
                    code: "L003".to_string(),
                    message: format!(
                        "Step '{}' depends_on '{}' which does not exist",
                        step.name, dep
                    ),
                });
            }
            deps.push(dep.clone());
        }
        if let Some(sub) = &step.parallel {
            validate_depends_on(sub, all_ids, path_str, diags, graph);
        }
        if let Some(sub) = &step.steps {
            validate_depends_on(sub, all_ids, path_str, diags, graph);
        }
    }
}

fn warn_empty_steps(steps: &[Step], path_str: &str, diags: &mut Vec<Diagnostic>) {
    for step in steps {
        let has_work = step.command.is_some()
            || step.parallel.is_some()
            || step.steps.is_some()
            || step.for_each.is_some();
        if !has_work {
            diags.push(Diagnostic {
                file: path_str.to_string(),
                line: None,
                severity: Severity::Warning,
                code: "L004".to_string(),
                message: format!(
                    "Step '{}' has no command, parallel, steps, or for_each",
                    step.name
                ),
            });
        }
        if let Some(sub) = &step.parallel {
            warn_empty_steps(sub, path_str, diags);
        }
        if let Some(sub) = &step.steps {
            warn_empty_steps(sub, path_str, diags);
        }
    }
}

/// Kahn's algorithm for topological sort / cycle detection.
/// Returns `Some(cycle_nodes)` if a cycle exists, `None` otherwise.
fn detect_cycle(graph: &HashMap<String, Vec<String>>) -> Option<Vec<String>> {
    // Build in-degree map
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for (node, deps) in graph {
        in_degree.entry(node.as_str()).or_insert(0);
        for dep in deps {
            in_degree.entry(dep.as_str()).or_insert(0);
        }
    }
    for deps in graph.values() {
        for dep in deps {
            *in_degree.entry(dep.as_str()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(n, _)| *n)
        .collect();

    let mut visited = 0;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(deps) = graph.get(node) {
            for dep in deps {
                let cnt = in_degree.entry(dep.as_str()).or_insert(0);
                if *cnt > 0 {
                    *cnt -= 1;
                    if *cnt == 0 {
                        queue.push_back(dep.as_str());
                    }
                }
            }
        }
    }

    if visited < in_degree.len() {
        // Cycle exists — collect the nodes still with non-zero in-degree
        let cycle_nodes: Vec<String> = in_degree
            .iter()
            .filter(|(_, d)| **d > 0)
            .map(|(n, _)| n.to_string())
            .collect();
        Some(cycle_nodes)
    } else {
        None
    }
}

// ── .raps-project linting ─────────────────────────────────────────────────────

/// Minimal shape of a `.raps-project` file.
#[derive(Debug, Deserialize)]
struct RapsProject {
    #[serde(default)]
    hub_id: String,
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    profile: Option<String>,
}

fn lint_raps_project(path: &Path, path_str: &str, diags: &mut Vec<Diagnostic>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic {
                file: path_str.to_string(),
                line: None,
                severity: Severity::Error,
                code: "L010".to_string(),
                message: format!("Cannot read .raps-project: {}", e),
            });
            return;
        }
    };

    // Try JSON first, then TOML
    let project: RapsProject = if let Ok(p) = serde_json::from_str(&content) {
        p
    } else if let Ok(p) = toml::from_str(&content) {
        p
    } else {
        diags.push(Diagnostic {
            file: path_str.to_string(),
            line: None,
            severity: Severity::Error,
            code: "L011".to_string(),
            message: "Failed to parse .raps-project (expected JSON or TOML)".to_string(),
        });
        return;
    };

    if project.hub_id.trim().is_empty() {
        diags.push(Diagnostic {
            file: path_str.to_string(),
            line: None,
            severity: Severity::Error,
            code: "L012".to_string(),
            message: "hub_id is empty or missing in .raps-project".to_string(),
        });
    }

    if project.project_id.trim().is_empty() {
        diags.push(Diagnostic {
            file: path_str.to_string(),
            line: None,
            severity: Severity::Error,
            code: "L013".to_string(),
            message: "project_id is empty or missing in .raps-project".to_string(),
        });
    }

    if let Some(profile) = &project.profile {
        // Verify the profile exists in the raps config
        if !profile_exists(profile) {
            diags.push(Diagnostic {
                file: path_str.to_string(),
                line: None,
                severity: Severity::Warning,
                code: "L014".to_string(),
                message: format!(
                    "Profile '{}' referenced in .raps-project not found in raps config",
                    profile
                ),
            });
        }
    }
}

/// Check whether a named profile exists in the raps configuration directory.
fn profile_exists(name: &str) -> bool {
    // raps stores profiles in ~/.config/raps/profiles/<name>.toml or similar
    let dirs = directories::ProjectDirs::from("com", "autodesk", "raps");
    let Some(dirs) = dirs else { return false };
    let profile_file = dirs
        .config_dir()
        .join("profiles")
        .join(format!("{}.toml", name));
    profile_file.exists()
}

// ── Secret scan linting ───────────────────────────────────────────────────────

fn lint_secrets(path: &Path, path_str: &str, diags: &mut Vec<Diagnostic>) {
    let matches = match secret_scan::scan_file(path) {
        Ok(m) => m,
        Err(_) => return, // Can't read or binary — skip
    };
    for m in matches {
        diags.push(Diagnostic {
            file: path_str.to_string(),
            line: Some(m.line_number),
            severity: Severity::Error,
            code: "L020".to_string(),
            message: format!(
                "Potential secret detected [{}]: {}",
                m.pattern_name, m.snippet
            ),
        });
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn print_table(r: &LintReport) {
    for d in &r.diagnostics {
        let sev = match d.severity {
            Severity::Error => "error".red().bold(),
            Severity::Warning => "warning".yellow().bold(),
        };
        let loc = match d.line {
            Some(l) => format!("{}:{}", d.file, l),
            None => d.file.clone(),
        };
        println!(
            "{}: [{}] {} — {}",
            loc.cyan(),
            d.code.dimmed(),
            sev,
            d.message
        );
    }

    println!();
    println!(
        "{} {} file(s) checked, {} warning(s), {} error(s)",
        if r.errors > 0 {
            "FAIL".red().bold()
        } else if r.warnings > 0 {
            "WARN".yellow().bold()
        } else {
            "OK".green().bold()
        },
        r.files_checked,
        r.warnings,
        r.errors
    );
}

fn print_csv(r: &LintReport) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    wtr.write_record(["file", "line", "severity", "code", "message"])?;
    for d in &r.diagnostics {
        wtr.write_record([
            &d.file,
            &d.line.map(|l| l.to_string()).unwrap_or_default(),
            &d.severity.to_string(),
            &d.code,
            &d.message,
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph(edges: &[(&str, &str)]) -> HashMap<String, Vec<String>> {
        let mut g: HashMap<String, Vec<String>> = HashMap::new();
        for (from, to) in edges {
            g.entry(from.to_string()).or_default().push(to.to_string());
            g.entry(to.to_string()).or_default();
        }
        g
    }

    #[test]
    fn test_no_cycle() {
        let g = make_graph(&[("a", "b"), ("b", "c")]);
        assert!(detect_cycle(&g).is_none());
    }

    #[test]
    fn test_cycle_detected() {
        let g = make_graph(&[("a", "b"), ("b", "c"), ("c", "a")]);
        assert!(detect_cycle(&g).is_some());
    }

    #[test]
    fn test_empty_graph() {
        let g: HashMap<String, Vec<String>> = HashMap::new();
        assert!(detect_cycle(&g).is_none());
    }

    #[test]
    fn test_collect_step_ids() {
        use crate::commands::pipeline::Step;
        let steps = vec![
            Step {
                name: "build".to_string(),
                id: Some("build-id".to_string()),
                ..Step::default()
            },
            Step {
                name: "test".to_string(),
                ..Step::default()
            },
        ];
        let mut ids = HashSet::new();
        collect_step_ids(&steps, &mut ids);
        assert!(ids.contains("build"));
        assert!(ids.contains("build-id"));
        assert!(ids.contains("test"));
    }
}
