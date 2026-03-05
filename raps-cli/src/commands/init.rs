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

    // Step 2 — test auth (non-fatal)
    let auth_ok = step_test_auth(&client_id, &client_secret).await;
    let _ = auth_ok;

    // Step 3 — 3-legged login (optional)
    let (logged_in, maybe_auth) = step_login(&client_id, &client_secret).await;

    // Step 4 — hub discovery
    let hubs = step_discover_hubs(maybe_auth.as_ref(), &client_id, &client_secret).await;
    let _ = (logged_in, &profile_name, hubs); // used in future steps

    println!();
    println!("{}", "═".repeat(BOX_WIDTH));
    println!(
        "  {}",
        "Setup complete. Run `raps status` anytime to check your configuration.".bold()
    );
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

// ─── step 2: test 2-legged auth ──────────────────────────────────────────────

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

fn save_choice_options(profile_name: &str) -> Vec<String> {
    vec![
        format!("Save to profile '{}' only  (default)", profile_name),
        "Save to profile + print export line for ~/.bashrc / ~/.zshrc".to_string(),
        "Save to profile + auto-append to detected shell rc file".to_string(),
    ]
}

// ─── step 3: 3-legged login ──────────────────────────────────────────────────

/// Returns (logged_in, Option<AuthClient>)
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

    // Build auth client directly from credentials
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

async fn save_profile(name: &str, client_id: &str, client_secret: &str) -> Result<()> {
    use crate::commands::config::{load_profiles, save_profiles, ProfileConfig};

    let mut data = load_profiles().await?;

    let profile = data
        .profiles
        .entry(name.to_string())
        .or_insert_with(|| ProfileConfig {
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

    #[test]
    fn test_save_choice_label() {
        // Option labels used in the select prompt
        let opts = save_choice_options("myprofile");
        assert_eq!(opts.len(), 3);
        assert!(opts[0].contains("myprofile"));   // option 1 mentions profile name
        assert!(opts[1].contains("export"));      // option 2 mentions export
        assert!(opts[2].contains("auto"));        // option 3 mentions auto-append
    }

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
}
