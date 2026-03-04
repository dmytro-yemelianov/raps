// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps status` — full context dashboard command.
//!
//! Prints a single-page summary of auth state, accessible hubs, and active
//! context variables.  All output goes to **stdout** (not stderr) since this
//! is the primary output of the command.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::context_banner::{BOX_WIDTH, HubTier, tier_from_extension, truncate};
use crate::output::OutputFormat;

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Print a full-width `═` rule with an optional centred label.
fn eq_rule() {
    println!("{}", "═".repeat(BOX_WIDTH));
}

/// Print a section header using `─` dashes.
/// Layout: `  {title} ` + `─` × (BOX_WIDTH - title.len() - 3)
fn section_rule(title: &str) {
    let title_chars = title.chars().count();
    // "  " (2) + title + " " (1) = title_chars + 3 chars consumed
    let dash_count = BOX_WIDTH.saturating_sub(title_chars + 3);
    let line = format!("  {} {}", title, "─".repeat(dash_count));
    println!("{}", line.bold());
}

/// Mask a credential string as `first4…last4` (or the full string if too short).
fn mask_id(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 8 {
        return s.to_string();
    }
    let prefix: String = chars.iter().take(4).collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{}…{}", prefix, suffix)
}

/// Format a remaining-seconds count as `NhNm` or `Nm` or `Ns`.
fn format_remaining(remaining_secs: i64) -> String {
    if remaining_secs <= 0 {
        return "expired".to_string();
    }
    let h = remaining_secs / 3600;
    let m = (remaining_secs % 3600) / 60;
    let s = remaining_secs % 60;
    if h > 0 {
        format!("{}h{}m", h, m)
    } else if m > 0 {
        format!("{}m", m)
    } else {
        format!("{}s", s)
    }
}

/// Strip the `b.` prefix from an enterprise hub ID to get the bare account UUID.
fn bare_account_id(hub_id: &str) -> &str {
    hub_id.strip_prefix("b.").unwrap_or(hub_id)
}

// ─── JSON output type ─────────────────────────────────────────────────────────

#[derive(Serialize, schemars::JsonSchema)]
struct StatusOutput {
    two_legged: bool,
    three_legged: bool,
    token_expires_in_secs: Option<i64>,
    profile: Option<String>,
    client_id_masked: String,
    hubs: Vec<HubStatusOutput>,
    context_account_id: Option<String>,
    context_hub_id: Option<String>,
    context_project_id: Option<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct HubStatusOutput {
    id: String,
    name: String,
    tier: String,
    region: Option<String>,
    admin_api_ready: bool,
}

// ─── main entry point ─────────────────────────────────────────────────────────

/// Run the `raps status` dashboard command.
pub async fn run_status(
    auth_client: &raps_kernel::auth::AuthClient,
    dm_client: &raps_dm::DataManagementClient,
    output_format: OutputFormat,
) -> Result<()> {
    // ── 1. Auth checks ──────────────────────────────────────────────────────
    let two_leg_ok = auth_client.test_auth().await.is_ok();
    let three_leg_ok = auth_client.is_logged_in().await;
    let token_expiry: Option<i64> = auth_client.get_token_expiry().await;

    let now_ts = chrono::Utc::now().timestamp();
    let remaining_secs: Option<i64> = token_expiry.map(|exp| exp - now_ts);

    // ── 2. Profile / client_id ──────────────────────────────────────────────
    let config = raps_kernel::config::Config::from_env_lenient()
        .unwrap_or_else(|_| auth_client.config().clone());

    let profile_name: Option<String> =
        raps_kernel::config::load_profiles()
            .ok()
            .and_then(|pd| pd.active_profile);

    let client_id_masked = if config.client_id.is_empty() {
        "(not set)".to_string()
    } else {
        mask_id(&config.client_id)
    };

    // ── 3. Hubs (best-effort — not logged in is fine) ───────────────────────
    let hubs_result = dm_client.list_hubs().await;
    let hubs = hubs_result.unwrap_or_default();

    // ── 4. Context env vars ─────────────────────────────────────────────────
    let ctx_account_id = std::env::var("APS_ACCOUNT_ID").ok();
    let ctx_hub_id = std::env::var("APS_HUB_ID").ok();
    let ctx_project_id = std::env::var("APS_PROJECT_ID").ok();

    // ── 5. Render ───────────────────────────────────────────────────────────
    match output_format {
        OutputFormat::Json | OutputFormat::Yaml | OutputFormat::Csv | OutputFormat::Plain => {
            // Structured output — build a JSON-serialisable struct.
            let hub_outputs: Vec<HubStatusOutput> = hubs
                .iter()
                .map(|h| {
                    let ext = h.attributes.extension.as_ref()
                        .and_then(|e| e.extension_type.as_deref());
                    let tier = tier_from_extension(ext);
                    let admin_api_ready = tier == HubTier::Enterprise;
                    HubStatusOutput {
                        id: h.id.clone(),
                        name: h.attributes.name.clone(),
                        tier: format!("{:?}", tier).to_lowercase(),
                        region: h.attributes.region.clone(),
                        admin_api_ready,
                    }
                })
                .collect();

            let out = StatusOutput {
                two_legged: two_leg_ok,
                three_legged: three_leg_ok,
                token_expires_in_secs: remaining_secs,
                profile: profile_name,
                client_id_masked,
                hubs: hub_outputs,
                context_account_id: ctx_account_id,
                context_hub_id: ctx_hub_id,
                context_project_id: ctx_project_id,
            };
            output_format.write(&out)?;
        }

        OutputFormat::Table => {
            // ── Banner top ──────────────────────────────────────────────────
            println!("{}", "═".repeat(BOX_WIDTH));
            println!("  {}", "RAPS Status".bold());
            println!("{}", "═".repeat(BOX_WIDTH));
            println!();

            // ── Auth section ────────────────────────────────────────────────
            section_rule("Auth");

            // 2-legged row
            let two_leg_str = if two_leg_ok {
                format!("{}  Available      (client credentials)", "✓".green().bold())
            } else {
                format!("{}  Not available  (check APS_CLIENT_ID / APS_CLIENT_SECRET)", "✗".red().bold())
            };
            println!("  {:<12}  {}", "2-legged".bold(), two_leg_str);

            // 3-legged row
            let three_leg_str = if three_leg_ok {
                let expiry_suffix = remaining_secs
                    .map(|secs| format!("  expires in {}", format_remaining(secs)))
                    .unwrap_or_default();
                format!("{}  Logged in{}", "✓".green().bold(), expiry_suffix)
            } else {
                format!("{}  Not logged in  (run: raps auth login)", "✗".red().bold())
            };
            println!("  {:<12}  {}", "3-legged".bold(), three_leg_str);

            // Profile row
            let profile_display = profile_name.as_deref().unwrap_or("(none)");
            println!(
                "  {:<12}  {:<20}  client_id: {}",
                "Profile".bold(),
                profile_display,
                client_id_masked
            );
            println!();

            // ── Hubs section ────────────────────────────────────────────────
            section_rule("Hubs");

            if hubs.is_empty() {
                if three_leg_ok {
                    println!(
                        "  {}",
                        "(no hubs found)".dimmed()
                    );
                } else {
                    println!(
                        "  {}",
                        "(not logged in — run `raps auth login` to see hubs)".dimmed()
                    );
                }
            } else {
                for hub in &hubs {
                    let ext = hub.attributes.extension.as_ref()
                        .and_then(|e| e.extension_type.as_deref());
                    let tier = tier_from_extension(ext);
                    let region = hub.attributes.region.as_deref().unwrap_or("--");

                    // Tier glyph + label
                    let (glyph, tier_label) = match tier {
                        HubTier::Personal   => ("○", "PERSONAL   "),
                        HubTier::Enterprise => ("◆", "ENTERPRISE "),
                        HubTier::Unknown    => ("?", "UNKNOWN    "),
                    };

                    let name_col = format!("{:<24}", truncate(&hub.attributes.name, 24));
                    let id_short = {
                        let id = &hub.id;
                        let chars: Vec<char> = id.chars().collect();
                        if chars.len() <= 12 {
                            id.clone()
                        } else {
                            let prefix: String = chars.iter().take(6).collect();
                            let suffix: String = chars[chars.len() - 4..].iter().collect();
                            format!("{}…{}", prefix, suffix)
                        }
                    };
                    let id_col = format!("{:<12}", id_short);
                    let region_col = format!("[{}]", region);

                    let hub_line = format!(
                        "  {} {}  {}  {}  {}",
                        glyph, tier_label, name_col, id_col, region_col
                    );

                    match tier {
                        HubTier::Personal   => println!("{}", hub_line.dimmed()),
                        HubTier::Enterprise => println!("{}", hub_line.cyan().bold()),
                        HubTier::Unknown    => println!("{}", hub_line.dimmed()),
                    }

                    // For enterprise hubs, print sub-info
                    if tier == HubTier::Enterprise {
                        let account_id = bare_account_id(&hub.id);
                        println!("{}",
                            format!("                └─ Admin API: {}  Account ID: {}",
                                "✓ ready".green(),
                                account_id
                            )
                        );
                    }
                }
            }
            println!();

            // ── Context section ─────────────────────────────────────────────
            section_rule("Context");

            let fmt_ctx = |val: &Option<String>, env_name: &str| -> String {
                match val {
                    Some(v) => format!("{:<42}  env:{}", truncate(v, 42), env_name),
                    None    => format!("{:<42}  env:{}", "(not set)", env_name),
                }
            };

            println!(
                "  {:<14}  {}",
                "account_id".bold(),
                fmt_ctx(&ctx_account_id, "APS_ACCOUNT_ID")
            );
            println!(
                "  {:<14}  {}",
                "hub_id".bold(),
                fmt_ctx(&ctx_hub_id, "APS_HUB_ID")
            );
            println!(
                "  {:<14}  {}",
                "project_id".bold(),
                fmt_ctx(&ctx_project_id, "APS_PROJECT_ID")
            );

            // ── Banner bottom ───────────────────────────────────────────────
            println!("{}", "═".repeat(BOX_WIDTH));
        }
    }

    Ok(())
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_id_long() {
        let masked = mask_id("RCM7ABCDEFGHYJYS");
        assert!(masked.contains('…'), "should contain ellipsis");
        assert!(masked.starts_with("RCM7"), "should start with first 4 chars");
        assert!(masked.ends_with("YJYS"), "should end with last 4 chars");
    }

    #[test]
    fn test_mask_id_short_passthrough() {
        let short = "ABCD";
        assert_eq!(mask_id(short), short);
    }

    #[test]
    fn test_format_remaining_hours() {
        assert_eq!(format_remaining(3700), "1h1m");
    }

    #[test]
    fn test_format_remaining_minutes() {
        assert_eq!(format_remaining(1620), "27m");
    }

    #[test]
    fn test_format_remaining_seconds() {
        assert_eq!(format_remaining(45), "45s");
    }

    #[test]
    fn test_format_remaining_expired() {
        assert_eq!(format_remaining(0), "expired");
        assert_eq!(format_remaining(-100), "expired");
    }

    #[test]
    fn test_bare_account_id_strips_prefix() {
        assert_eq!(
            bare_account_id("b.01fb1602-2ec0-4b05-bf6e-39dc70b3ae05"),
            "01fb1602-2ec0-4b05-bf6e-39dc70b3ae05"
        );
    }

    #[test]
    fn test_bare_account_id_no_prefix() {
        assert_eq!(bare_account_id("a.personalid"), "a.personalid");
    }
}
