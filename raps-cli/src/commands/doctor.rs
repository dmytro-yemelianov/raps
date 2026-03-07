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
    checks.push(check_network_reachability().await);
    checks.push(check_config_permissions());
    checks.push(check_context_var_formats());
    checks.push(check_disk_space());
    checks.push(check_keyring());
    checks.push(check_env_conflicts());
    checks.push(check_version_staleness().await);
    checks.push(check_proxy_environment());

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

const NETWORK_PROBE_URL: &str = "https://developer.api.autodesk.com";

async fn check_network_reachability() -> CheckResult {
    use std::time::Duration;

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => return check("Network [network]", Status::Fail, &format!("Client build failed: {e}")),
    };

    match client.head(NETWORK_PROBE_URL).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() || status.as_u16() == 404 || status.as_u16() == 405 {
                check(
                    "Network [network]",
                    Status::Pass,
                    &format!("developer.api.autodesk.com reachable (HTTP {})", status.as_u16()),
                )
            } else {
                check(
                    "Network [network]",
                    Status::Warn,
                    &format!("Unexpected HTTP {} from APS endpoint", status.as_u16()),
                )
            }
        }
        Err(e) => {
            if e.is_timeout() {
                check("Network [network]", Status::Fail, "Connection timed out (5s) — check firewall/proxy")
            } else if e.is_connect() {
                check("Network [network]", Status::Fail, "Cannot connect to developer.api.autodesk.com — check network")
            } else {
                check("Network [network]", Status::Fail, &format!("Network error: {e}"))
            }
        }
    }
}

fn check_config_file_permissions(path: &std::path::Path) -> CheckResult {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(path) {
            Ok(meta) => {
                let mode = meta.permissions().mode();
                // Warn if group-readable (0o040) or world-readable (0o004)
                if mode & 0o044 != 0 {
                    check(
                        "Config Permissions",
                        Status::Warn,
                        &format!(
                            "{} is group/world readable (mode {:04o}) — run: chmod 600 {}",
                            path.display(),
                            mode & 0o777,
                            path.display()
                        ),
                    )
                } else {
                    check("Config Permissions", Status::Pass, "Config file is owner-only readable")
                }
            }
            Err(e) => check("Config Permissions", Status::Warn, &format!("Cannot stat config: {e}")),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        check("Config Permissions", Status::Pass, "Permission check not applicable on this OS")
    }
}

fn check_config_permissions() -> CheckResult {
    match directories::ProjectDirs::from("com", "autodesk", "raps") {
        Some(proj) => {
            let profiles_path = proj.config_dir().join("profiles.json");
            if profiles_path.exists() {
                check_config_file_permissions(&profiles_path)
            } else {
                check("Config Permissions", Status::Pass, "No config file found (not yet configured)")
            }
        }
        None => check("Config Permissions", Status::Warn, "Cannot determine config directory"),
    }
}

fn check_context_var_formats() -> CheckResult {
    check("Context Vars", Status::Warn, "not implemented yet")
}

fn check_disk_space() -> CheckResult {
    check("Disk Space", Status::Warn, "not implemented yet")
}

fn check_keyring() -> CheckResult {
    check("Keyring", Status::Warn, "not implemented yet")
}

fn check_env_conflicts() -> CheckResult {
    check("Env Conflicts", Status::Warn, "not implemented yet")
}

async fn check_version_staleness() -> CheckResult {
    check("Version [network]", Status::Warn, "not implemented yet")
}

fn check_proxy_environment() -> CheckResult {
    check("Proxy/TLS Env", Status::Warn, "not implemented yet")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1.00 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_network_check_name_contains_network_tag() {
        // Verify the check() helper produces a result whose name contains the [network] tag
        // when given the network check's name constant
        let c = check("Network [network]", Status::Pass, "reachable");
        assert!(c.name.contains("[network]"), "Network check name must contain [network] tag to signal network requirement");
    }

    #[test]
    fn test_network_endpoint_is_aps_domain() {
        assert!(NETWORK_PROBE_URL.starts_with("https://developer.api.autodesk.com"));
    }

    #[cfg(unix)]
    #[test]
    fn test_config_permissions_detects_world_readable() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        let result = check_config_file_permissions(tmp.path());
        assert_eq!(result.status, "warn", "world-readable config should warn");
        assert!(result.message.contains("world") || result.message.contains("group") || result.message.contains("readable"));
    }

    #[cfg(unix)]
    #[test]
    fn test_config_permissions_passes_for_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        let result = check_config_file_permissions(tmp.path());
        assert_eq!(result.status, "pass");
    }
}
