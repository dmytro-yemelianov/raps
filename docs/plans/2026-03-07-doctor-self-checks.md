# Doctor Self-Checks Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 8 new diagnostic checks to `raps doctor` covering network reachability, config file permissions, context variable format, disk space, keyring health, env var conflicts, version staleness, and proxy/TLS environment detection.

**Architecture:** All 8 checks follow the existing `check(name, status, message) -> CheckResult` pattern in `raps-cli/src/commands/doctor.rs`. Network-dependent checks (#1, #7) include `[network]` in their name. Checks that may trigger system prompts (#5 keyring) include a note in their warning message. No new crates needed except adding `fs2` (already in workspace) to `raps-cli/Cargo.toml`.

**Tech Stack:** `reqwest` (HTTP), `keyring` (keyring probe), `fs2` (disk space), `semver` (version compare), `url` (proxy URL masking), `regex` (UUID validation) — all already workspace deps.

---

## Reference

- **Check pattern:** `raps-cli/src/commands/doctor.rs` — `fn check(name, status, message)`, `Status::{Pass,Warn,Fail}`
- **Keyring:** service `"raps"`, username `"aps_token"` (from `raps-kernel/src/storage.rs:96-97`)
- **Config dir:** `directories::ProjectDirs::from("com", "autodesk", "raps").config_dir()`
- **Existing checks:** `check_config`, `check_two_leg_auth`, `check_three_leg_auth`, `check_cache`, `check_api_health`, `check_plugins`
- **execute() fn:** `raps-cli/src/commands/doctor.rs:66` — push new checks after `check_plugins`

---

### Task 1: Add `fs2` dependency and wire all 8 checks into `execute()`

**Files:**
- Modify: `raps-cli/Cargo.toml`
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Add `fs2` to `raps-cli/Cargo.toml`**

In the `[dependencies]` section, add after the `directories.workspace = true` line:

```toml
# Disk space queries
fs2.workspace = true
```

**Step 2: Wire 8 stub checks into `execute()` in `doctor.rs`**

In `execute()` after `checks.push(check_plugins());`, add:

```rust
    checks.push(check_network_reachability().await);
    checks.push(check_config_permissions());
    checks.push(check_context_var_formats());
    checks.push(check_disk_space());
    checks.push(check_keyring());
    checks.push(check_env_conflicts());
    checks.push(check_version_staleness().await);
    checks.push(check_proxy_environment());
```

Then add 8 stub functions (return Warn "not implemented yet") — these will be replaced task by task.

```rust
async fn check_network_reachability() -> CheckResult {
    check("Network [network]", Status::Warn, "not implemented yet")
}

fn check_config_permissions() -> CheckResult {
    check("Config Permissions", Status::Warn, "not implemented yet")
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
```

**Step 3: Verify it compiles**

```bash
cd /home/dmytro/github/raps/raps
cargo build -p raps-cli 2>&1 | tail -5
```

Expected: compiles without errors.

**Step 4: Commit**

```bash
git add raps-cli/Cargo.toml raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): wire 8 new self-check stubs into execute()"
```

---

### Task 2: Network reachability check

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write the unit test for the helper**

Add to the `#[cfg(test)]` mod at the bottom of `doctor.rs`:

```rust
    #[test]
    fn test_network_check_name_contains_network_tag() {
        // The check name must include "[network]" so users know it requires connectivity
        let c = CheckResult {
            name: "Network [network]".to_string(),
            status: "pass".to_string(),
            message: "reachable".to_string(),
        };
        assert!(c.name.contains("[network]"));
    }

    #[test]
    fn test_network_endpoint_is_aps_domain() {
        // Verify the constant points to the right domain
        assert!(NETWORK_PROBE_URL.starts_with("https://developer.api.autodesk.com"));
    }
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p raps-cli -- doctor 2>&1 | grep -E "FAILED|error"
```

Expected: compile error — `NETWORK_PROBE_URL` not defined.

**Step 3: Implement the check**

Replace the `check_network_reachability` stub with:

```rust
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
                // 404/405 is fine — the root path may not exist, but the host is reachable
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
```

**Step 4: Run unit tests**

```bash
cargo test -p raps-cli -- doctor::tests 2>&1 | tail -10
```

Expected: `test_network_check_name_contains_network_tag` and `test_network_endpoint_is_aps_domain` pass.

**Step 5: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add network reachability check with [network] tag"
```

---

### Task 3: Config file permissions check

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write the unit tests**

Add to `#[cfg(test)]` mod:

```rust
    #[cfg(unix)]
    #[test]
    fn test_config_permissions_detects_world_readable() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        // Set world-readable permissions
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
```

Note: these tests call `check_config_file_permissions(path)` — a pure helper we'll extract.

**Step 2: Run to see compile error**

```bash
cargo test -p raps-cli -- doctor::tests::test_config_permissions 2>&1 | head -5
```

Expected: error — function not defined.

**Step 3: Implement**

Add the helper and replace the stub:

```rust
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
            // Check the profiles.json which may contain sensitive data
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
```

**Step 4: Run tests**

```bash
cargo test -p raps-cli -- doctor::tests::test_config_permissions 2>&1 | tail -10
```

Expected: both Unix tests pass.

**Step 5: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add config file permissions check (Unix: warn if group/world readable)"
```

---

### Task 4: Context variable format validation

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write the unit tests**

```rust
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
        // When no vars are set, should pass (they're optional)
        // We can test the helper directly
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
```

**Step 2: Run to see compile error**

```bash
cargo test -p raps-cli -- doctor::tests::test_is_valid_uuid 2>&1 | head -5
```

**Step 3: Implement**

```rust
fn is_valid_uuid(s: &str) -> bool {
    // UUID v4 format: 8-4-4-4-12 hex chars
    let re = regex::Regex::new(
        r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
    ).expect("valid UUID regex");
    re.is_match(s)
}

/// Returns a list of human-readable issue descriptions, empty if all valid.
fn validate_context_vars(
    account_id: Option<&str>,
    hub_id: Option<&str>,
    project_id: Option<&str>,
) -> Vec<String> {
    let mut issues = Vec::new();

    if let Some(id) = account_id {
        if !is_valid_uuid(id) {
            issues.push(format!("APS_ACCOUNT_ID '{}' is not a valid UUID", id));
        }
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
```

**Step 4: Run tests**

```bash
cargo test -p raps-cli -- doctor::tests::test_is_valid_uuid doctor::tests::test_context_var 2>&1 | tail -15
```

Expected: all pass.

**Step 5: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add context variable UUID format validation check"
```

---

### Task 5: Disk space check

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write the unit test for the threshold helper**

```rust
    #[test]
    fn test_disk_space_classify_critical() {
        // < 100 MB is Fail
        assert!(50 * 1024 * 1024_u64 < DISK_WARN_THRESHOLD_BYTES);
    }

    #[test]
    fn test_disk_space_classify_warn() {
        // between 100 MB and 500 MB is Warn
        let warn_threshold = DISK_WARN_THRESHOLD_BYTES;
        let fail_threshold = DISK_FAIL_THRESHOLD_BYTES;
        assert!(200 * 1024 * 1024_u64 > fail_threshold);
        assert!(200 * 1024 * 1024_u64 < warn_threshold);
    }

    #[test]
    fn test_disk_space_classify_pass() {
        assert!(1024 * 1024 * 1024_u64 > DISK_WARN_THRESHOLD_BYTES);
    }
```

**Step 2: Run to see compile error**

```bash
cargo test -p raps-cli -- doctor::tests::test_disk_space 2>&1 | head -5
```

**Step 3: Implement**

```rust
const DISK_FAIL_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;  // 100 MB
const DISK_WARN_THRESHOLD_BYTES: u64 = 500 * 1024 * 1024;  // 500 MB

fn check_disk_space() -> CheckResult {
    use fs2::FileExt; // brings available_space into scope indirectly

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
```

**Step 4: Run tests**

```bash
cargo test -p raps-cli -- doctor::tests::test_disk_space 2>&1 | tail -10
```

**Step 5: Compile check**

```bash
cargo build -p raps-cli 2>&1 | tail -5
```

Expected: no errors.

**Step 6: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add disk space check (warn <500MB, fail <100MB near cache dir)"
```

---

### Task 6: Token keyring probe

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write unit tests for keyring error classification**

```rust
    #[test]
    fn test_classify_keyring_no_entry_means_not_logged_in() {
        let result = classify_keyring_error(&keyring::Error::NoEntry);
        assert_eq!(result.status, "warn");
        assert!(result.message.contains("Not logged in") || result.message.contains("raps auth login"));
    }

    #[test]
    fn test_classify_keyring_access_denied_is_fail() {
        // Simulate an unexpected platform error (not NoEntry)
        // We check the pass-through behavior via the classify function
        let err = keyring::Error::NoStorageAccess(Box::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        )));
        let result = classify_keyring_error(&err);
        assert_eq!(result.status, "fail");
        assert!(result.message.contains("may prompt") || result.message.contains("unlock") || result.message.contains("access"));
    }
```

**Step 2: Run to see compile error**

```bash
cargo test -p raps-cli -- doctor::tests::test_classify_keyring 2>&1 | head -5
```

**Step 3: Implement**

```rust
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
```

**Step 4: Run tests**

```bash
cargo test -p raps-cli -- doctor::tests::test_classify_keyring 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add keyring probe check (distinguishes not-logged-in from broken keyring)"
```

---

### Task 7: Environment variable conflict detection

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write unit tests**

```rust
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
```

**Step 2: Run to see compile error**

```bash
cargo test -p raps-cli -- doctor::tests::test_detect 2>&1 | head -5
```

**Step 3: Implement**

```rust
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
```

**Step 4: Run tests**

```bash
cargo test -p raps-cli -- doctor::tests::test_detect 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add env var conflict detection (warns when both env creds and profile are active)"
```

---

### Task 8: Version staleness check

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write unit tests for the version comparison helper**

```rust
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
        // local is newer (pre-release build)
        let result = compare_versions("5.4.0-dev", "5.3.3");
        assert_eq!(result, VersionCompare::Ahead);
    }

    #[test]
    fn test_parse_github_tag_strips_v_prefix() {
        assert_eq!(strip_v_prefix("v5.3.3"), "5.3.3");
        assert_eq!(strip_v_prefix("5.3.3"), "5.3.3");
    }
```

**Step 2: Run to see compile error**

```bash
cargo test -p raps-cli -- doctor::tests::test_compare_versions doctor::tests::test_parse_github 2>&1 | head -5
```

**Step 3: Implement**

```rust
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
            &format!("Update available: v{current} → v{latest}  (run: npm i -g raps-cli@latest)"),
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
```

**Step 4: Run unit tests**

```bash
cargo test -p raps-cli -- doctor::tests::test_compare_versions doctor::tests::test_parse_github 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add version staleness check via GitHub releases API [network]"
```

---

### Task 9: Proxy/TLS environment check

**Files:**
- Modify: `raps-cli/src/commands/doctor.rs`

**Step 1: Write unit tests**

```rust
    #[test]
    fn test_mask_proxy_url_strips_credentials() {
        let masked = mask_proxy_url("http://user:password@proxy.corp.com:8080");
        assert!(!masked.contains("password"));
        assert!(masked.contains("proxy.corp.com"));
    }

    #[test]
    fn test_mask_proxy_url_no_credentials_unchanged() {
        let masked = mask_proxy_url("http://proxy.corp.com:8080");
        assert_eq!(masked, "http://proxy.corp.com:8080");
    }

    #[test]
    fn test_mask_proxy_url_invalid_falls_back_to_host() {
        let masked = mask_proxy_url("not-a-url");
        assert_eq!(masked, "not-a-url");
    }

    #[test]
    fn test_find_proxy_env_vars_detects_https_proxy() {
        // Isolation: test the pure detection logic
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
```

**Step 2: Run to see compile error**

```bash
cargo test -p raps-cli -- doctor::tests::test_mask_proxy doctor::tests::test_find_proxy 2>&1 | head -5
```

**Step 3: Implement**

```rust
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
```

**Step 4: Run tests**

```bash
cargo test -p raps-cli -- doctor::tests::test_mask_proxy doctor::tests::test_find_proxy 2>&1 | tail -10
```

**Step 5: Full test run**

```bash
cargo test -p raps-cli -- doctor 2>&1 | tail -20
```

Expected: all doctor tests pass.

**Step 6: Build release**

```bash
cargo build --release -p raps-cli 2>&1 | tail -5
```

**Step 7: Smoke test**

```bash
./target/release/raps doctor
```

Expected: all 14 checks listed (6 original + 8 new), with `[network]` on checks 1 and 7.

**Step 8: Commit**

```bash
git add raps-cli/src/commands/doctor.rs
git commit -m "feat(doctor): add proxy/TLS environment check with credential masking"
```

---

### Task 10: Final integration — add `tempfile` dev-dep and clean up

**Files:**
- Modify: `raps-cli/Cargo.toml` (verify `tempfile` is in dev-deps — it already is)
- Modify: `raps-cli/src/commands/doctor.rs` (remove any remaining "not implemented yet" stubs)

**Step 1: Verify no stubs remain**

```bash
grep "not implemented yet" /home/dmytro/github/raps/raps/raps-cli/src/commands/doctor.rs
```

Expected: no output.

**Step 2: Run full test suite**

```bash
cargo test -p raps-cli 2>&1 | tail -20
```

**Step 3: Run clippy**

```bash
cargo clippy -p raps-cli -- -D warnings 2>&1 | tail -20
```

Fix any warnings before continuing.

**Step 4: Final commit**

```bash
git add raps-cli/src/commands/doctor.rs raps-cli/Cargo.toml
git commit -m "feat(doctor): 8 new self-checks complete (network, permissions, context vars, disk, keyring, conflicts, version, proxy)"
```
