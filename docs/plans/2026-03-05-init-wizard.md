# `raps init` — Setup Wizard Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `raps init` command that walks users through APS app creation, credential setup, 2-legged auth test, 3-legged login, hub discovery, enterprise context setup, and final status summary.

**Architecture:** Linear state machine in `raps-cli/src/commands/init.rs`. Each step is a private async fn; state (profile name, credentials, hub list) passed as local variables. The wizard builds its own `AuthClient` / `DataManagementClient` after credentials are saved, so it does not depend on any pre-built clients from `main.rs`.

**Tech Stack:** `raps_kernel::prompts` (dialoguer), `raps_kernel::config` (Config), `crate::commands::config::{load_profiles, save_profiles, ProfileConfig, ProfilesData}`, `crate::context_banner::{ContextBanner, print_warning_no_enterprise, HubTier, tier_from_extension, BOX_WIDTH}`, `crate::commands::status::run_status`, `colored`, `raps_dm::DataManagementClient`

---

### Task 1: Scaffold `init.rs` with step-1 (credentials) and wire command

**Files:**
- Create: `raps-cli/src/commands/init.rs`
- Modify: `raps-cli/src/commands/mod.rs`
- Modify: `raps-cli/src/main.rs`

**Step 1: Write the failing test**

Add to `init.rs` (unit tests for the helpers):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shell_rc_bash() {
        // If SHELL contains "bash", should return ".bashrc"
        let rc = shell_rc_filename("bash");
        assert_eq!(rc, ".bashrc");
    }

    #[test]
    fn test_detect_shell_rc_zsh() {
        let rc = shell_rc_filename("zsh");
        assert_eq!(rc, ".zshrc");
    }

    #[test]
    fn test_detect_shell_rc_fish() {
        let rc = shell_rc_filename("fish");
        // fish uses config.fish, not a .rc file
        assert_eq!(rc, ".profile");
    }

    #[test]
    fn test_detect_shell_rc_unknown() {
        let rc = shell_rc_filename("unknown_shell");
        assert_eq!(rc, ".profile");
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init 2>&1 | tail -10
```
Expected: compile error (module not found)

**Step 3: Create `init.rs` scaffold with `shell_rc_filename` helper**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps init` — first-time setup wizard.
//!
//! Walks the user through: APS credentials → 2-legged auth test → 3-legged
//! login → hub discovery → enterprise context setup → status summary.

use anyhow::Result;
use colored::Colorize;

use crate::context_banner::BOX_WIDTH;

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Return the shell rc filename for a given shell binary path or name.
fn shell_rc_filename(shell: &str) -> &'static str {
    if shell.contains("zsh") {
        ".zshrc"
    } else if shell.contains("bash") {
        ".bashrc"
    } else {
        ".profile"
    }
}

/// Print a step header: `────… [N/6] Title ────…` (BOX_WIDTH chars total)
fn step_header(n: u8, total: u8, title: &str) {
    println!();
    let label = format!("  [{}/{}] {}  ", n, total, title);
    let label_len = label.chars().count();
    let dashes = "─".repeat(BOX_WIDTH.saturating_sub(label_len));
    println!("{}", format!("{}{}", label, dashes).bold());
}

// ─── entry point ─────────────────────────────────────────────────────────────

/// Run the `raps init` wizard.
pub async fn run_init() -> Result<()> {
    // Banner
    println!("{}", "═".repeat(BOX_WIDTH));
    println!("  {}", "RAPS Init — First-time setup".bold());
    println!("{}", "═".repeat(BOX_WIDTH));
    println!();
    println!("  This wizard will configure your APS credentials, test authentication,");
    println!("  log you in, and set up hub context.");
    println!();
    println!("  Steps:");
    println!("    [1] APS Credentials");
    println!("    [2] Test 2-Legged Auth");
    println!("    [3] 3-Legged Login");
    println!("    [4] Hub Discovery");
    println!("    [5] Enterprise Context  (if enterprise hub found)");
    println!("    [6] Summary");

    // Step 1 — credentials
    let (profile_name, client_id, client_secret) = step_credentials().await?;

    // Steps 2-6 use a fresh AuthClient built from the saved profile
    // (remaining steps are stubs for now — will be filled in Tasks 2-6)
    let _ = (&profile_name, &client_id, &client_secret);

    println!();
    println!("{}", "═".repeat(BOX_WIDTH));
    println!("  {}", "Setup complete. Run `raps status` anytime to check your configuration.".bold());
    println!("{}", "═".repeat(BOX_WIDTH));

    Ok(())
}

// ─── step 1: credentials ─────────────────────────────────────────────────────

async fn step_credentials() -> Result<(String, String, String)> {
    step_header(1, 6, "APS Credentials");
    println!();
    println!("  Create an APS app (if you haven't yet):");
    println!("    {} {}", "→".cyan(), "https://aps.autodesk.com/myapps".underline());
    println!();

    let profile_name = raps_kernel::prompts::spawn_prompt(|| {
        raps_kernel::prompts::input("  Profile name", Some("main"))
    })
    .await?;

    let client_id = raps_kernel::prompts::spawn_prompt(|| {
        raps_kernel::prompts::input_validated(
            "  Client ID",
            None,
            |s: &String| {
                if s.trim().is_empty() {
                    Err("Client ID cannot be empty")
                } else {
                    Ok(())
                }
            },
        )
    })
    .await?;

    let client_secret = raps_kernel::prompts::spawn_prompt(|| {
        raps_kernel::prompts::input_validated(
            "  Client Secret",
            None,
            |s: &String| {
                if s.trim().is_empty() {
                    Err("Client Secret cannot be empty")
                } else {
                    Ok(())
                }
            },
        )
    })
    .await?;

    // Save profile
    save_profile(&profile_name, &client_id, &client_secret).await?;

    println!();
    println!(
        "  {} Profile '{}' saved",
        "✓".green().bold(),
        profile_name.cyan()
    );

    Ok((profile_name, client_id, client_secret))
}

async fn save_profile(name: &str, client_id: &str, client_secret: &str) -> Result<()> {
    use crate::commands::config::{load_profiles, save_profiles, ProfileConfig};

    let mut data = load_profiles().await?;

    let profile = data.profiles.entry(name.to_string()).or_insert_with(|| ProfileConfig {
        client_id: None,
        client_secret: None,
        base_url: None,
        callback_url: None,
        da_nickname: None,
        use_keychain: None,
        context_hub_id: None,
        context_project_id: None,
        context_account_id: None,
    });
    profile.client_id = Some(client_id.to_string());
    profile.client_secret = Some(client_secret.to_string());

    data.active_profile = Some(name.to_string());
    save_profiles(&data).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shell_rc_bash() {
        let rc = shell_rc_filename("bash");
        assert_eq!(rc, ".bashrc");
    }

    #[test]
    fn test_detect_shell_rc_zsh() {
        let rc = shell_rc_filename("zsh");
        assert_eq!(rc, ".zshrc");
    }

    #[test]
    fn test_detect_shell_rc_fish() {
        let rc = shell_rc_filename("fish");
        assert_eq!(rc, ".profile");
    }

    #[test]
    fn test_detect_shell_rc_unknown() {
        let rc = shell_rc_filename("unknown_shell");
        assert_eq!(rc, ".profile");
    }
}
```

**Step 4: Add `pub mod init;` to `commands/mod.rs`**

In `raps-cli/src/commands/mod.rs`, add alongside the other `pub mod` declarations:
```rust
pub mod init;
```

**Step 5: Add `Init` variant and dispatch to `main.rs`**

Find the `Status` variant near line 233 and add `Init` just before it:
```rust
/// First-time setup wizard — configure credentials, login, and hub context
Init,
```

In `command_name()` (near line 978), add alongside `Commands::Status`:
```rust
Commands::Init => "init",
```

In `execute_command()` match (near line 1099), add:
```rust
Commands::Init => {
    commands::init::run_init().await?;
}
```

In `GROUPED_COMMANDS_HELP` (near line 105), add to the Utility section:
```
  init          First-time setup wizard (credentials, login, hub context)
```

**Step 6: Run tests**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init -- --nocapture 2>&1 | tail -20
```
Expected: 4 tests pass (`test_detect_shell_rc_*`)

**Step 7: Verify it compiles**

```bash
cd /root/github/raps/raps && cargo build -p raps-cli 2>&1 | tail -10
```
Expected: no errors

**Step 8: Commit**

```bash
git add raps-cli/src/commands/init.rs raps-cli/src/commands/mod.rs raps-cli/src/main.rs
git commit -m "feat(init): scaffold raps init wizard with step 1 (credentials)"
```

---

### Task 2: Step 2 — Test 2-Legged Auth

**Files:**
- Modify: `raps-cli/src/commands/init.rs`

**Step 1: Write the failing test**

Add to the `tests` module in `init.rs`:
```rust
#[test]
fn test_step_auth_result_formatting() {
    // Test the message strings we'll print (pure formatting, no network)
    let ok_msg = format!("{} 2-legged auth OK", "✓".green().bold());
    assert!(ok_msg.contains("2-legged auth OK"));

    let fail_msg = format!(
        "{} 2-legged auth failed — check client_id / client_secret",
        "✗".red().bold()
    );
    assert!(fail_msg.contains("2-legged auth failed"));
}
```

**Step 2: Run test to verify it passes immediately (pure formatting test)**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init::tests::test_step_auth_result 2>&1 | tail -10
```
Expected: PASS (no network calls)

**Step 3: Implement `step_test_auth`**

Add to `init.rs` after `step_credentials`:

```rust
// ─── step 2: test 2-legged auth ──────────────────────────────────────────────

async fn step_test_auth(client_id: &str, client_secret: &str) -> bool {
    step_header(2, 6, "Test 2-Legged Auth");
    println!();
    println!("  Testing client credentials...");

    let config = raps_kernel::config::Config::from_credentials(client_id, client_secret);
    let auth = raps_kernel::auth::AuthClient::new(config);

    match auth.test_auth().await {
        Ok(()) => {
            println!("  {} 2-legged auth OK", "✓".green().bold());
            true
        }
        Err(e) => {
            println!(
                "  {} 2-legged auth failed — check client_id / client_secret",
                "✗".red().bold()
            );
            println!("  {}", format!("  {e}").dimmed());
            println!(
                "  {} You can still continue and fix credentials later.",
                "!".yellow()
            );
            false
        }
    }
}
```

**Step 4: Check `Config::from_credentials` exists — if not, use `Config::from_env_lenient` with env override**

```bash
grep -n "from_credentials\|fn new\|pub fn" /root/github/raps/raps/raps-kernel/src/config.rs | head -20
```

If `from_credentials` does not exist, use this instead:
```rust
async fn step_test_auth(client_id: &str, client_secret: &str) -> bool {
    step_header(2, 6, "Test 2-Legged Auth");
    println!();
    println!("  Testing client credentials...");

    // Build a minimal config directly (avoids env var dependency)
    let config = raps_kernel::config::Config {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: "http://localhost:8080/callback".to_string(),
        da_nickname: None,
        http_config: raps_kernel::http::HttpClientConfig::default(),
    };
    let auth = raps_kernel::auth::AuthClient::new(config);

    match auth.test_auth().await {
        Ok(()) => {
            println!("  {} 2-legged auth OK", "✓".green().bold());
            true
        }
        Err(e) => {
            println!(
                "  {} 2-legged auth failed — check client_id / client_secret",
                "✗".red().bold()
            );
            println!("  {}", format!("    {e}").dimmed());
            println!(
                "  {} You can still continue and fix credentials later.",
                "!".yellow()
            );
            false
        }
    }
}
```

**Step 5: Wire into `run_init`**

Replace the placeholder `// Steps 2-6 use a fresh AuthClient built from the saved profile` block:

```rust
// Step 2 — test auth (non-fatal)
let auth_ok = step_test_auth(&client_id, &client_secret).await;
let _ = auth_ok; // used in future steps
```

**Step 6: Run all init tests**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init 2>&1 | tail -15
```
Expected: all tests pass

**Step 7: Commit**

```bash
git add raps-cli/src/commands/init.rs
git commit -m "feat(init): add step 2 — test 2-legged auth"
```

---

### Task 3: Steps 3 & 4 — 3-Legged Login + Hub Discovery

**Files:**
- Modify: `raps-cli/src/commands/init.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_save_choice_label() {
    // Option labels used in the select prompt
    let opts = save_choice_options("myprofile");
    assert_eq!(opts.len(), 3);
    assert!(opts[0].contains("myprofile"));   // option 1 mentions profile name
    assert!(opts[1].contains("export"));      // option 2 mentions export
    assert!(opts[2].contains("auto"));        // option 3 mentions auto-append
}
```

Add `save_choice_options` function (used in step 5, defined now for testability):
```rust
fn save_choice_options(profile_name: &str) -> Vec<String> {
    vec![
        format!("Save to profile '{}' only  (default)", profile_name),
        "Save to profile + print export line for ~/.bashrc / ~/.zshrc".to_string(),
        "Save to profile + auto-append to detected shell rc file".to_string(),
    ]
}
```

**Step 2: Run test to verify it fails**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init::tests::test_save_choice_label 2>&1 | tail -10
```
Expected: compile error (function not defined yet)

**Step 3: Add `save_choice_options` to `init.rs` and implement steps 3 & 4**

```rust
fn save_choice_options(profile_name: &str) -> Vec<String> {
    vec![
        format!("Save to profile '{}' only  (default)", profile_name),
        "Save to profile + print export line for ~/.bashrc / ~/.zshrc".to_string(),
        "Save to profile + auto-append to detected shell rc file".to_string(),
    ]
}

// ─── step 3: 3-legged login ──────────────────────────────────────────────────

/// Returns (logged_in, AuthClient)
async fn step_login(
    client_id: &str,
    client_secret: &str,
) -> (bool, Option<raps_kernel::auth::AuthClient>) {
    step_header(3, 6, "3-Legged Login");
    println!();
    println!("  Log in to access hubs and user context.");

    let proceed = raps_kernel::prompts::spawn_prompt(|| {
        raps_kernel::prompts::confirm("  Proceed with browser login?", true)
    })
    .await
    .unwrap_or(false);

    if !proceed {
        println!("  {} Skipped.", "→".dimmed());
        return (false, None);
    }

    // Build auth client (config loaded from profile env fallback)
    let config = raps_kernel::config::Config {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: "http://localhost:8080/callback".to_string(),
        da_nickname: None,
        http_config: raps_kernel::http::HttpClientConfig::default(),
    };
    let auth = raps_kernel::auth::AuthClient::new(config);

    let scopes = &[
        "data:read", "data:write", "data:create", "data:search",
        "bucket:read", "account:read", "user:read",
    ];

    let use_device = raps_kernel::interactive::is_headless();
    if use_device {
        println!(
            "  {} Headless environment detected — using device code flow.",
            "!".yellow()
        );
    }

    let result = if use_device {
        auth.login_device(scopes).await
    } else {
        auth.login(scopes).await
    };

    match result {
        Ok(_token) => {
            // Try to get user info for the greeting
            if let Ok(user) = auth.get_user_info().await {
                let name = user.name.or(user.preferred_username).unwrap_or_default();
                let email = user.email.unwrap_or_default();
                println!(
                    "  {} Logged in as {} ({})",
                    "✓".green().bold(),
                    name.cyan().bold(),
                    email
                );
            } else {
                println!("  {} Logged in", "✓".green().bold());
            }
            (true, Some(auth))
        }
        Err(e) => {
            println!("  {} Login failed: {}", "✗".red().bold(), e);
            println!(
                "  {} Continuing without 3-legged auth. Run `raps auth login` later.",
                "!".yellow()
            );
            (false, None)
        }
    }
}

// ─── step 4: hub discovery ───────────────────────────────────────────────────

async fn step_discover_hubs(
    auth: Option<&raps_kernel::auth::AuthClient>,
    client_id: &str,
    client_secret: &str,
) -> Vec<raps_dm::types::Hub> {
    step_header(4, 6, "Hub Discovery");
    println!();

    let auth_ref = match auth {
        Some(a) => a,
        None => {
            println!("  {} Skipped (not logged in).", "→".dimmed());
            return vec![];
        }
    };

    let config = raps_kernel::config::Config {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: "http://localhost:8080/callback".to_string(),
        da_nickname: None,
        http_config: raps_kernel::http::HttpClientConfig::default(),
    };

    let dm = raps_dm::DataManagementClient::new(config, auth_ref.clone());

    match dm.list_hubs().await {
        Ok(hubs) if !hubs.is_empty() => {
            let banner = crate::context_banner::ContextBanner::from_hubs(&hubs);
            banner.print_inline();
            hubs
        }
        Ok(_) => {
            println!("  {}", "(no hubs found — check your account access)".dimmed());
            vec![]
        }
        Err(e) => {
            println!("  {} Could not list hubs: {}", "✗".red().bold(), e);
            vec![]
        }
    }
}
```

**Step 4: Wire into `run_init`**

Add after `step_test_auth`:
```rust
// Step 3 — 3-legged login (optional)
let (logged_in, maybe_auth) = step_login(&client_id, &client_secret).await;
let _ = logged_in;

// Step 4 — hub discovery
let hubs = step_discover_hubs(maybe_auth.as_ref(), &client_id, &client_secret).await;
let _ = hubs;
```

**Step 5: Check `DataManagementClient::new` signature**

```bash
grep -n "pub fn new\b" /root/github/raps/raps/raps-dm/src/lib.rs | head -5
```

Adjust the constructor call if signature differs.

**Step 6: Run all init tests**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init 2>&1 | tail -15
```
Expected: all tests pass

**Step 7: Commit**

```bash
git add raps-cli/src/commands/init.rs
git commit -m "feat(init): add steps 3 (login) and 4 (hub discovery)"
```

---

### Task 4: Step 5 — Enterprise Context

**Files:**
- Modify: `raps-cli/src/commands/init.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_export_line_format() {
    let line = export_line("01fb1602-2ec0-4b05-bf6e-39dc70b3ae05");
    assert_eq!(
        line,
        "export APS_ACCOUNT_ID=01fb1602-2ec0-4b05-bf6e-39dc70b3ae05"
    );
}
```

**Step 2: Run to verify it fails**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init::tests::test_export_line_format 2>&1 | tail -5
```
Expected: compile error (function not defined)

**Step 3: Implement `step_enterprise_context` and helpers**

```rust
fn export_line(account_id: &str) -> String {
    format!("export APS_ACCOUNT_ID={}", account_id)
}

// ─── step 5: enterprise context ──────────────────────────────────────────────

async fn step_enterprise_context(
    hubs: &[raps_dm::types::Hub],
    profile_name: &str,
) -> Result<()> {
    use crate::context_banner::{ContextBanner, HubTier, tier_from_extension};

    step_header(5, 6, "Enterprise Context");
    println!();

    // Find enterprise hubs
    let enterprise_hubs: Vec<&raps_dm::types::Hub> = hubs
        .iter()
        .filter(|h| {
            let ext = h.attributes.extension.as_ref()
                .and_then(|e| e.extension_type.as_deref());
            tier_from_extension(ext) == HubTier::Enterprise
        })
        .collect();

    if enterprise_hubs.is_empty() {
        crate::context_banner::print_warning_no_enterprise();
        println!();
        println!("  To get an enterprise hub, register your app in ACC Custom Integrations:");
        println!("    {} {}", "→".cyan(), "https://acc.autodesk.com");
        println!("    {} (Account Admin → Custom Integrations)", " ".repeat(2));
        println!("    {} {}", "→".cyan(), "Docs: rapscli.xyz/docs/custom-integrations");
        return Ok(());
    }

    // Show enterprise hub(s) — if multiple, use first
    let hub = enterprise_hubs[0];
    let raw_id = &hub.id;
    let account_id = raw_id.strip_prefix("b.").unwrap_or(raw_id);
    let region = hub.attributes.region.as_deref();

    let banner = ContextBanner::from_account(account_id, &hub.attributes.name, region);
    banner.print_box();

    println!();
    println!("  To use admin commands, register this app in ACC Custom Integrations:");
    println!("    {} {}", "→".cyan(), "https://acc.autodesk.com".underline());
    println!("    {} (Account Admin → Custom Integrations)", " ".repeat(2));
    println!("    {} {}", "→".cyan(), "Docs: rapscli.xyz/docs/custom-integrations".underline());
    println!();
    println!(
        "  Save {} = {}",
        "APS_ACCOUNT_ID".bold(),
        account_id.cyan()
    );
    println!();

    // Prompt save choice
    let options = save_choice_options(profile_name);
    let choice = raps_kernel::prompts::spawn_prompt(move || {
        raps_kernel::prompts::select_with_default("  How would you like to save it?", &options, 0)
    })
    .await
    .unwrap_or(0);

    // Always save to profile
    save_account_to_profile(profile_name, account_id).await?;
    println!(
        "  {} Saved to profile '{}'",
        "✓".green().bold(),
        profile_name.cyan()
    );

    match choice {
        1 => {
            // Print export line
            let line = export_line(account_id);
            println!();
            println!("  Add this to your shell config:");
            println!("    {}", line.cyan().bold());
        }
        2 => {
            // Auto-append to shell rc
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
            let rc_file = shell_rc_filename(&shell);
            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            let rc_path = std::path::PathBuf::from(&home).join(rc_file);
            let line = format!("\n{}\n", export_line(account_id));
            match std::fs::OpenOptions::new().create(true).append(true).open(&rc_path) {
                Ok(mut f) => {
                    use std::io::Write;
                    f.write_all(line.as_bytes())?;
                    println!(
                        "  {} Appended to {}",
                        "✓".green().bold(),
                        rc_path.display().to_string().cyan()
                    );
                    println!(
                        "  {} Reload with: source {}",
                        "→".dimmed(),
                        rc_path.display()
                    );
                }
                Err(e) => {
                    println!("  {} Could not write to {}: {}", "✗".red().bold(), rc_path.display(), e);
                    println!("  Add manually: {}", export_line(account_id).cyan());
                }
            }
        }
        _ => {} // choice 0: profile only, already done
    }

    Ok(())
}

async fn save_account_to_profile(profile_name: &str, account_id: &str) -> Result<()> {
    use crate::commands::config::{load_profiles, save_profiles};

    let mut data = load_profiles().await?;
    if let Some(profile) = data.profiles.get_mut(profile_name) {
        profile.context_account_id = Some(account_id.to_string());
        save_profiles(&data).await?;
    }
    Ok(())
}
```

**Step 4: Wire into `run_init`**

Replace `let _ = hubs;` with:
```rust
// Step 5 — enterprise context (only if hubs were discovered)
if !hubs.is_empty() {
    step_enterprise_context(&hubs, &profile_name).await?;
} else if logged_in {
    step_header(5, 6, "Enterprise Context");
    println!();
    println!("  {} Skipped (no hubs found).", "→".dimmed());
}
```

**Step 5: Run test**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init 2>&1 | tail -15
```
Expected: all tests pass including `test_export_line_format`

**Step 6: Commit**

```bash
git add raps-cli/src/commands/init.rs
git commit -m "feat(init): add step 5 — enterprise context setup"
```

---

### Task 5: Step 6 — Summary + complete run_init

**Files:**
- Modify: `raps-cli/src/commands/init.rs`

**Step 1: Wire step 6 (summary) into `run_init`**

Step 6 reuses `run_status` directly. Add this import at the top of `init.rs`:
```rust
use crate::commands::status::run_status;
use crate::output::OutputFormat;
```

Add `step_summary` function:
```rust
// ─── step 6: summary ─────────────────────────────────────────────────────────

async fn step_summary(client_id: &str, client_secret: &str, logged_in: bool) {
    step_header(6, 6, "Summary");
    println!();

    // Build fresh clients from the saved profile (env vars may not be set yet,
    // so build directly from the credentials we have)
    let config = raps_kernel::config::Config {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: "http://localhost:8080/callback".to_string(),
        da_nickname: None,
        http_config: raps_kernel::http::HttpClientConfig::default(),
    };

    let auth = raps_kernel::auth::AuthClient::new(config.clone());
    let dm = raps_dm::DataManagementClient::new(config, auth.clone());

    if let Err(e) = run_status(&auth, &dm, OutputFormat::Table).await {
        println!("  {} Could not load status: {}", "!".yellow(), e);
    }

    let _ = logged_in; // informational, status already shows it
}
```

Wire into `run_init` — replace the final `println!` block with:
```rust
// Step 6 — summary
step_summary(&client_id, &client_secret, logged_in).await;

println!();
println!("{}", "═".repeat(BOX_WIDTH));
println!(
    "  {}",
    "Setup complete. Run `raps status` anytime to check your configuration."
        .bold()
);
println!("{}", "═".repeat(BOX_WIDTH));
```

**Step 2: Run all init tests**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli init 2>&1 | tail -15
```
Expected: all tests pass

**Step 3: Build release binary and smoke-test**

```bash
cd /root/github/raps/raps && cargo build -p raps-cli 2>&1 | tail -5
./target/debug/raps init --help
```
Expected: shows "First-time setup wizard" description

**Step 4: Commit**

```bash
git add raps-cli/src/commands/init.rs
git commit -m "feat(init): add step 6 (summary) and complete run_init flow"
```

---

### Task 6: Polish + full test pass

**Files:**
- Modify: `raps-cli/src/commands/init.rs`

**Step 1: Run full raps-cli test suite**

```bash
cd /root/github/raps/raps && cargo test -p raps-cli 2>&1 | tail -20
```
Expected: all existing tests pass; no regressions

**Step 2: Run clippy**

```bash
cd /root/github/raps/raps && cargo clippy -p raps-cli -- -D warnings 2>&1 | tail -20
```
Fix any clippy warnings.

**Step 3: Rebuild release binary and verify `raps init --help`**

```bash
cargo build --release -p raps-cli 2>&1 | tail -5
./target/release/raps --help | grep init
./target/release/raps init --help
```
Expected: `init` appears in help output with correct description

**Step 4: Verify `raps init` appears in `raps --help` grouped output**

```bash
./target/release/raps --help | grep -A5 "Utility"
```
Expected: `init` listed under Utility section

**Step 5: Commit + copy binary**

```bash
cp -u target/release/raps ~/.cargo/bin/raps
git add raps-cli/src/commands/init.rs
git commit -m "feat(init): polish — pass clippy, verify help output"
```
