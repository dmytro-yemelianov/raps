// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Diagnostic doctor command.
//!
//! Runs a suite of health checks and reports pass/warn/fail for each.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

#[derive(Serialize, schemars::JsonSchema)]
struct DoctorOutput {
    checks: Vec<CheckResult>,
    summary: DoctorSummary,
}

#[derive(Serialize, schemars::JsonSchema)]
struct CheckResult {
    name: String,
    status: String,
    message: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct DoctorSummary {
    passed: usize,
    warnings: usize,
    failed: usize,
}

enum Status {
    Pass,
    Warn,
    Fail,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Warn => "warn",
            Status::Fail => "fail",
        }
    }

    fn icon(&self) -> String {
        match self {
            Status::Pass => "✓".green().bold().to_string(),
            Status::Warn => "!".yellow().bold().to_string(),
            Status::Fail => "✗".red().bold().to_string(),
        }
    }
}

fn check(name: &str, status: Status, message: &str) -> CheckResult {
    CheckResult {
        name: name.to_string(),
        status: status.as_str().to_string(),
        message: message.to_string(),
    }
}

pub async fn execute(output_format: OutputFormat) -> Result<()> {
    let mut checks = Vec::new();

    checks.push(check_config());
    checks.push(check_two_leg_auth().await);
    checks.push(check_three_leg_auth().await);
    checks.push(check_cache());
    checks.push(check_api_health());
    checks.push(check_plugins());

    let passed = checks.iter().filter(|c| c.status == "pass").count();
    let warnings = checks.iter().filter(|c| c.status == "warn").count();
    let failed = checks.iter().filter(|c| c.status == "fail").count();

    let summary = DoctorSummary {
        passed,
        warnings,
        failed,
    };

    let output = DoctorOutput { checks, summary };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "raps doctor".bold());
            println!("{}", "─".repeat(60));

            for c in &output.checks {
                let icon = match c.status.as_str() {
                    "pass" => Status::Pass.icon(),
                    "warn" => Status::Warn.icon(),
                    _ => Status::Fail.icon(),
                };
                println!("  {} {:<24} {}", icon, c.name, c.message.dimmed());
            }

            println!("{}", "─".repeat(60));
            let failed_str = if failed > 0 {
                failed.to_string().red().to_string()
            } else {
                failed.to_string()
            };
            println!(
                "  {} passed, {} warnings, {} failed",
                passed.to_string().green(),
                warnings.to_string().yellow(),
                failed_str,
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

fn check_config() -> CheckResult {
    match raps_kernel::config::Config::from_env_lenient() {
        Ok(config) => {
            if config.require_credentials().is_ok() {
                check("Configuration", Status::Pass, "Credentials configured")
            } else {
                check(
                    "Configuration",
                    Status::Warn,
                    "No client_id/client_secret set",
                )
            }
        }
        Err(e) => check("Configuration", Status::Fail, &format!("Load error: {e}")),
    }
}

async fn check_two_leg_auth() -> CheckResult {
    let config = match raps_kernel::config::Config::from_env_lenient() {
        Ok(c) => c,
        Err(_) => return check("2-Legged Auth", Status::Fail, "Config not available"),
    };

    if config.require_credentials().is_err() {
        return check("2-Legged Auth", Status::Warn, "No credentials configured");
    }

    let auth = raps_kernel::auth::AuthClient::new(config);
    match auth.test_auth().await {
        Ok(()) => check("2-Legged Auth", Status::Pass, "Token acquired successfully"),
        Err(e) => check("2-Legged Auth", Status::Fail, &format!("{e}")),
    }
}

async fn check_three_leg_auth() -> CheckResult {
    let config = match raps_kernel::config::Config::from_env_lenient() {
        Ok(c) => c,
        Err(_) => return check("3-Legged Auth", Status::Fail, "Config not available"),
    };

    let auth = raps_kernel::auth::AuthClient::new(config);
    if auth.is_logged_in().await {
        if let Some(expiry) = auth.get_token_expiry().await {
            let now = chrono::Utc::now().timestamp();
            let remaining = expiry - now;
            if remaining > 0 {
                let hours = remaining / 3600;
                let minutes = (remaining % 3600) / 60;
                check(
                    "3-Legged Auth",
                    Status::Pass,
                    &format!("Logged in (expires in {hours}h {minutes}m)"),
                )
            } else {
                check(
                    "3-Legged Auth",
                    Status::Warn,
                    "Token expired (will auto-refresh)",
                )
            }
        } else {
            check("3-Legged Auth", Status::Pass, "Logged in")
        }
    } else {
        check(
            "3-Legged Auth",
            Status::Warn,
            "Not logged in (run: raps auth login)",
        )
    }
}

fn check_cache() -> CheckResult {
    match raps_kernel::cache::cache_dir() {
        Ok(dir) => {
            if !raps_kernel::cache::is_enabled() {
                return check("Cache", Status::Warn, "Cache is disabled");
            }
            match raps_kernel::cache::stats() {
                Ok((count, size)) => {
                    let size_str = format_size(size);
                    let writable = std::fs::create_dir_all(&dir).is_ok() || dir.exists();
                    if writable {
                        check(
                            "Cache",
                            Status::Pass,
                            &format!("{count} entries, {size_str}"),
                        )
                    } else {
                        check(
                            "Cache",
                            Status::Fail,
                            &format!("Directory not writable: {}", dir.display()),
                        )
                    }
                }
                Err(e) => check("Cache", Status::Fail, &format!("Stats error: {e}")),
            }
        }
        Err(e) => check("Cache", Status::Fail, &format!("Dir error: {e}")),
    }
}

fn check_api_health() -> CheckResult {
    use raps_kernel::api_health::HealthStatus;

    let snap = raps_kernel::api_health::snapshot();
    match snap.health_status {
        HealthStatus::Healthy => check(
            "API Health",
            Status::Pass,
            &format!(
                "avg {}ms, {} samples",
                snap.avg_latency.as_millis(),
                snap.sample_count
            ),
        ),
        HealthStatus::Degraded => check(
            "API Health",
            Status::Warn,
            &format!(
                "avg {}ms, jitter {}ms ({} failures)",
                snap.avg_latency.as_millis(),
                snap.jitter.as_millis(),
                snap.failure_count
            ),
        ),
        HealthStatus::Unhealthy => check(
            "API Health",
            Status::Fail,
            &format!(
                "avg {}ms, {} failures",
                snap.avg_latency.as_millis(),
                snap.failure_count
            ),
        ),
        HealthStatus::Unknown => check("API Health", Status::Warn, "No API calls recorded yet"),
    }
}

fn check_plugins() -> CheckResult {
    let pm = match crate::plugins::PluginManager::new() {
        Ok(pm) => pm,
        Err(e) => return check("Plugins", Status::Warn, &format!("Cannot load: {e}")),
    };
    let plugins = pm.list_plugins();
    if plugins.is_empty() {
        check("Plugins", Status::Pass, "No plugins installed")
    } else {
        let count = plugins.len();
        let mut untrusted = 0;
        for p in &plugins {
            if let Ok(result) = pm.verify_plugin(&p.name)
                && !result.trusted
            {
                untrusted += 1;
            }
        }
        if untrusted > 0 {
            check(
                "Plugins",
                Status::Warn,
                &format!("{count} plugin(s), {untrusted} untrusted"),
            )
        } else {
            check(
                "Plugins",
                Status::Pass,
                &format!("{count} plugin(s), all verified"),
            )
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
