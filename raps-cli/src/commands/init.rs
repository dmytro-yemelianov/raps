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
}
