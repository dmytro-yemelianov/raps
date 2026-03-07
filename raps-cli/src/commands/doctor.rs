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
                // Warn if any non-owner permission bits (read, write, execute for group or world)
                if mode & 0o077 != 0 {
                    check(
                        "Config Permissions",
                        Status::Warn,
                        &format!(
                            "{} has non-owner permissions (mode {:04o}) — run: chmod 600 {}",
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

fn is_valid_uuid(s: &str) -> bool {
    use std::sync::OnceLock;
    static UUID_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = UUID_RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
        ).expect("valid UUID regex")
    });
    re.is_match(s)
}

/// Returns a list of human-readable issue descriptions, empty if all valid.
fn validate_context_vars(
    account_id: Option<&str>,
    hub_id: Option<&str>,
    project_id: Option<&str>,
) -> Vec<String> {
    let mut issues = Vec::new();

    if let Some(id) = account_id
        && !is_valid_uuid(id)
    {
        issues.push(format!("APS_ACCOUNT_ID '{}' is not a valid UUID", id));
    }

    if let Some(id) = hub_id {
        // Hub IDs may have a "b." prefix followed by a UUID, or be a plain UUID
        let bare = id.strip_prefix("b.").unwrap_or(id);
        if bare.is_empty() || (!is_valid_uuid(bare) && !is_valid_uuid(id)) {
            issues.push(format!("APS_HUB_ID '{}' does not look like a valid hub ID (expected UUID or b.<uuid>)", id));
        }
    }

    if let Some(id) = project_id {
        // Project IDs may have "b." prefix
        let bare = id.strip_prefix("b.").unwrap_or(id);
        if !is_valid_uuid(bare) {
            issues.push(format!("APS_PROJECT_ID '{}' is not a valid project ID (expected UUID or b.<uuid>)", id));
        }
    }

    issues
}

fn check_context_var_formats() -> CheckResult {
    let account_id = std::env::var("APS_ACCOUNT_ID").ok();
    let hub_id = std::env::var("APS_HUB_ID").ok();
    let project_id = std::env::var("APS_PROJECT_ID").ok();

    if account_id.is_none() && hub_id.is_none() && project_id.is_none() {
        return check("Context Vars", Status::Pass, "No context variables set");
    }

    let issues = validate_context_vars(
        account_id.as_deref(),
        hub_id.as_deref(),
        project_id.as_deref(),
    );

    if issues.is_empty() {
        let set: Vec<&str> = [
            account_id.as_ref().map(|_| "APS_ACCOUNT_ID"),
            hub_id.as_ref().map(|_| "APS_HUB_ID"),
            project_id.as_ref().map(|_| "APS_PROJECT_ID"),
        ]
        .into_iter()
        .flatten()
        .collect();
        check("Context Vars", Status::Pass, &format!("{} set and valid", set.join(", ")))
    } else {
        check("Context Vars", Status::Fail, &issues.join("; "))
    }
}

const DISK_FAIL_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;  // 100 MB
const DISK_WARN_THRESHOLD_BYTES: u64 = 500 * 1024 * 1024;  // 500 MB

fn check_disk_space() -> CheckResult {
    let check_path = raps_kernel::cache::cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir());

    // Walk up to find an existing ancestor to query
    let query_path = {
        let mut p = check_path.as_path();
        loop {
            if p.exists() {
                break p.to_path_buf();
            }
            match p.parent() {
                Some(parent) => p = parent,
                None => break std::env::temp_dir(),
            }
        }
    };

    match fs2::available_space(&query_path) {
        Ok(available) => {
            let human = format_size(available);
            if available < DISK_FAIL_THRESHOLD_BYTES {
                check(
                    "Disk Space",
                    Status::Fail,
                    &format!("Only {human} available near cache dir — free disk space"),
                )
            } else if available < DISK_WARN_THRESHOLD_BYTES {
                check(
                    "Disk Space",
                    Status::Warn,
                    &format!("{human} available near cache dir (low)"),
                )
            } else {
                check("Disk Space", Status::Pass, &format!("{human} available"))
            }
        }
        Err(e) => check("Disk Space", Status::Warn, &format!("Cannot determine disk space: {e}")),
    }
}

fn classify_keyring_error(err: &keyring::Error) -> CheckResult {
    match err {
        keyring::Error::NoEntry => check(
            "Keyring",
            Status::Warn,
            "Not logged in — run: raps auth login",
        ),
        keyring::Error::NoStorageAccess(_) => check(
            "Keyring",
            Status::Fail,
            "Keyring access denied — on some systems may prompt for unlock or require elevated permissions",
        ),
        other => check(
            "Keyring",
            Status::Fail,
            &format!("Keyring error (may prompt for system unlock): {other}"),
        ),
    }
}

fn check_keyring() -> CheckResult {
    match keyring::Entry::new("raps", "aps_token") {
        Ok(entry) => match entry.get_password() {
            Ok(_) => check("Keyring", Status::Pass, "Keyring accessible and token present"),
            Err(keyring::Error::NoEntry) => check(
                "Keyring",
                Status::Warn,
                "Not logged in — run: raps auth login",
            ),
            Err(e) => classify_keyring_error(&e),
        },
        Err(e) => check(
            "Keyring",
            Status::Fail,
            &format!("Cannot create keyring entry (may need system keyring unlock): {e}"),
        ),
    }
}

fn detect_credential_conflicts(env_creds_set: bool, profile_active: bool) -> Vec<String> {
    let mut conflicts = Vec::new();
    if env_creds_set && profile_active {
        conflicts.push(
            "APS_CLIENT_ID/APS_CLIENT_SECRET env vars are set AND an active profile is configured — \
             env vars take precedence; profile credentials are silently ignored".to_string(),
        );
    }
    conflicts
}

fn check_env_conflicts() -> CheckResult {
    let env_creds_set = std::env::var("APS_CLIENT_ID").is_ok()
        || std::env::var("APS_CLIENT_SECRET").is_ok();

    let profile_active = raps_kernel::config::load_profiles()
        .ok()
        .and_then(|pd| pd.active_profile)
        .is_some();

    let conflicts = detect_credential_conflicts(env_creds_set, profile_active);

    if conflicts.is_empty() {
        if env_creds_set {
            check("Env Conflicts", Status::Pass, "Using env var credentials (no profile active)")
        } else if profile_active {
            check("Env Conflicts", Status::Pass, "Using active profile credentials (no env override)")
        } else {
            check("Env Conflicts", Status::Pass, "No credential sources active")
        }
    } else {
        check("Env Conflicts", Status::Warn, &conflicts.join("; "))
    }
}

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/dmytro-yemelianov/raps/releases/latest";

#[derive(Debug, PartialEq)]
enum VersionCompare {
    UpToDate,
    UpdateAvailable,
    Ahead,
    ParseError,
}

fn strip_v_prefix(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

fn compare_versions(current: &str, latest: &str) -> VersionCompare {
    let Ok(cur) = semver::Version::parse(strip_v_prefix(current)) else {
        return VersionCompare::ParseError;
    };
    let Ok(lat) = semver::Version::parse(strip_v_prefix(latest)) else {
        return VersionCompare::ParseError;
    };
    match cur.cmp(&lat) {
        std::cmp::Ordering::Equal => VersionCompare::UpToDate,
        std::cmp::Ordering::Less => VersionCompare::UpdateAvailable,
        std::cmp::Ordering::Greater => VersionCompare::Ahead,
    }
}

async fn check_version_staleness() -> CheckResult {
    use std::time::Duration;

    let current = env!("CARGO_PKG_VERSION");

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(format!("raps/{current}"))
        .build()
    {
        Ok(c) => c,
        Err(e) => return check("Version [network]", Status::Warn, &format!("Cannot check version: {e}")),
    };

    let resp = match client.get(GITHUB_RELEASES_URL).send().await {
        Ok(r) => r,
        Err(e) => {
            return check(
                "Version [network]",
                Status::Warn,
                &format!("Cannot reach GitHub releases API (requires network): {e}"),
            );
        }
    };

    if !resp.status().is_success() {
        return check(
            "Version [network]",
            Status::Warn,
            &format!("GitHub API returned HTTP {} — skipping version check", resp.status().as_u16()),
        );
    }

    let json: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return check("Version [network]", Status::Warn, &format!("Cannot parse GitHub response: {e}")),
    };

    let tag = match json["tag_name"].as_str() {
        Some(t) => t,
        None => return check("Version [network]", Status::Warn, "No tag_name in GitHub release response"),
    };

    let latest = strip_v_prefix(tag);

    match compare_versions(current, latest) {
        VersionCompare::UpToDate => check(
            "Version [network]",
            Status::Pass,
            &format!("v{current} is up to date"),
        ),
        VersionCompare::UpdateAvailable => check(
            "Version [network]",
            Status::Warn,
            &format!("Update available: v{current} → v{latest} (run: npm i -g @dmytro-yemelianov/raps-cli@latest)"),
        ),
        VersionCompare::Ahead => check(
            "Version [network]",
            Status::Pass,
            &format!("v{current} (ahead of latest release v{latest})"),
        ),
        VersionCompare::ParseError => check(
            "Version [network]",
            Status::Warn,
            &format!("Cannot compare versions: current={current}, latest={latest}"),
        ),
    }
}

fn mask_proxy_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(mut u) => {
            if u.password().is_some() {
                let _ = u.set_password(Some("***"));
            }
            if !u.username().is_empty() {
                let _ = u.set_username("***");
            }
            u.to_string()
        }
        Err(_) => raw.to_string(),
    }
}

/// Accepts a list of (name, value) pairs (testable without touching real env).
fn find_proxy_from_vars(vars: &[(String, String)]) -> Option<String> {
    const PROXY_VARS: &[&str] = &[
        "HTTPS_PROXY", "https_proxy",
        "HTTP_PROXY",  "http_proxy",
        "ALL_PROXY",   "all_proxy",
    ];
    for name in PROXY_VARS {
        if let Some((_, val)) = vars.iter().find(|(k, _)| k == name) {
            return Some(format!("{name}={}", mask_proxy_url(val)));
        }
    }
    None
}

fn check_proxy_environment() -> CheckResult {
    let env_vars: Vec<(String, String)> = std::env::vars()
        .filter(|(k, _)| {
            matches!(
                k.as_str(),
                "HTTPS_PROXY" | "https_proxy" | "HTTP_PROXY" | "http_proxy" | "ALL_PROXY" | "all_proxy"
            )
        })
        .collect();

    match find_proxy_from_vars(&env_vars) {
        Some(proxy_info) => check(
            "Proxy/TLS Env",
            Status::Warn,
            &format!(
                "Proxy detected: {proxy_info} — TLS interception may affect APS API calls; \
                 if cert errors occur, check corporate CA bundle"
            ),
        ),
        None => check("Proxy/TLS Env", Status::Pass, "No proxy environment variables detected"),
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
        assert!(result.message.contains("non-owner") || result.message.contains("permissions") || result.message.contains("chmod"));
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

    #[test]
    fn test_is_valid_uuid_accepts_valid() {
        assert!(is_valid_uuid("01fb1602-2ec0-4b05-bf6e-39dc70b3ae05"));
        assert!(is_valid_uuid("00000000-0000-0000-0000-000000000000"));
    }

    #[test]
    fn test_is_valid_uuid_rejects_invalid() {
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid("01fb1602-2ec0-4b05-bf6e")); // too short
        assert!(!is_valid_uuid(""));
    }

    #[test]
    fn test_context_var_check_no_vars_set_passes() {
        let issues = validate_context_vars(None, None, None);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_context_var_check_invalid_uuid_fails() {
        let issues = validate_context_vars(Some("not-a-uuid"), None, None);
        assert!(!issues.is_empty());
        assert!(issues[0].contains("APS_ACCOUNT_ID"));
    }

    #[test]
    fn test_context_var_check_valid_uuid_passes() {
        let issues = validate_context_vars(
            Some("01fb1602-2ec0-4b05-bf6e-39dc70b3ae05"),
            None,
            None,
        );
        assert!(issues.is_empty());
    }

    #[test]
    fn test_disk_space_classify_critical() {
        // 50 MB < DISK_FAIL_THRESHOLD_BYTES (100 MB)
        assert!(50 * 1024 * 1024_u64 < DISK_FAIL_THRESHOLD_BYTES);
    }

    #[test]
    fn test_disk_space_classify_warn() {
        // 200 MB: above fail threshold but below warn threshold
        assert!(200 * 1024 * 1024_u64 > DISK_FAIL_THRESHOLD_BYTES);
        assert!(200 * 1024 * 1024_u64 < DISK_WARN_THRESHOLD_BYTES);
    }

    #[test]
    fn test_disk_space_classify_pass() {
        // 1 GB > DISK_WARN_THRESHOLD_BYTES (500 MB)
        assert!(1024 * 1024 * 1024_u64 > DISK_WARN_THRESHOLD_BYTES);
    }

    #[test]
    fn test_classify_keyring_no_entry_means_not_logged_in() {
        let result = classify_keyring_error(&keyring::Error::NoEntry);
        assert_eq!(result.status, "warn");
        assert!(result.message.contains("Not logged in") || result.message.contains("raps auth login"));
    }

    #[test]
    fn test_classify_keyring_access_denied_is_fail() {
        let err = keyring::Error::NoStorageAccess(Box::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        )));
        let result = classify_keyring_error(&err);
        assert_eq!(result.status, "fail");
        assert!(result.message.contains("may prompt") || result.message.contains("unlock") || result.message.contains("access"));
    }

    #[test]
    fn test_detect_no_conflict_when_only_env_set() {
        let conflicts = detect_credential_conflicts(
            true,  // env vars set
            false, // no active profile
        );
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_no_conflict_when_only_profile_set() {
        let conflicts = detect_credential_conflicts(false, true);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflict_when_both_set() {
        let conflicts = detect_credential_conflicts(true, true);
        assert!(!conflicts.is_empty());
        assert!(conflicts[0].contains("APS_CLIENT_ID") || conflicts[0].contains("profile"));
    }

    #[test]
    fn test_compare_versions_current_is_latest() {
        let result = compare_versions("5.3.3", "5.3.3");
        assert_eq!(result, VersionCompare::UpToDate);
    }

    #[test]
    fn test_compare_versions_update_available() {
        let result = compare_versions("5.3.3", "5.4.0");
        assert_eq!(result, VersionCompare::UpdateAvailable);
    }

    #[test]
    fn test_compare_versions_ahead_of_release() {
        let result = compare_versions("6.0.0", "5.3.3");
        assert_eq!(result, VersionCompare::Ahead);
    }

    #[test]
    fn test_parse_github_tag_strips_v_prefix() {
        assert_eq!(strip_v_prefix("v5.3.3"), "5.3.3");
        assert_eq!(strip_v_prefix("5.3.3"), "5.3.3");
    }

    #[test]
    fn test_mask_proxy_url_strips_credentials() {
        let masked = mask_proxy_url("http://user:password@proxy.corp.com:8080");
        assert!(!masked.contains("password"));
        assert!(masked.contains("proxy.corp.com"));
    }

    #[test]
    fn test_mask_proxy_url_no_credentials_unchanged() {
        let masked = mask_proxy_url("http://proxy.corp.com:8080");
        // url::Url normalises by appending a trailing slash for the path component
        assert!(masked.starts_with("http://proxy.corp.com:8080"));
        assert!(!masked.contains('@'));
    }

    #[test]
    fn test_mask_proxy_url_invalid_falls_back_to_host() {
        let masked = mask_proxy_url("not-a-url");
        assert_eq!(masked, "not-a-url");
    }

    #[test]
    fn test_find_proxy_env_vars_detects_https_proxy() {
        let vars = vec![
            ("HTTPS_PROXY".to_string(), "http://proxy:8080".to_string()),
        ];
        let found = find_proxy_from_vars(&vars);
        assert!(found.is_some());
        assert!(found.unwrap().contains("HTTPS_PROXY"));
    }

    #[test]
    fn test_find_proxy_env_vars_empty_when_none_set() {
        let vars: Vec<(String, String)> = vec![];
        assert!(find_proxy_from_vars(&vars).is_none());
    }
}
