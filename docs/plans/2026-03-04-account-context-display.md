# Account Context Display Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Personal vs Enterprise hub distinction visually explicit at every relevant surface — inline line before hub commands, bordered box before admin commands, warning box when no enterprise hub found, and a new `raps status` dashboard command.

**Architecture:** A new `context_banner.rs` module in raps-cli holds all classification and rendering logic. Commands opt-in by constructing a `ContextBanner` and calling the appropriate render method to stderr. `raps status` is a new top-level command wired into `execute_command()` alongside `Doctor`. All output targets 80-column width; banners only print in table (interactive) mode.

**Tech Stack:** Rust, `colored` crate (already in workspace), `raps-dm` Hub types (already imported in relevant commands).

---

### Task 1: Create `context_banner.rs` — HubTier classification

**Files:**
- Create: `raps-cli/src/context_banner.rs`

**Step 1: Write the failing test**

Add at the bottom of the new file:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Visual context banner for Personal vs Enterprise hub display.

/// Width of the terminal box borders (total line width)
pub const BOX_WIDTH: usize = 80;
/// Inner content width (BOX_WIDTH minus borders and padding: │  …  │)
pub const INNER_WIDTH: usize = 76;

/// Hub account tier — determines visual treatment.
#[derive(Debug, Clone, PartialEq)]
pub enum HubTier {
    Personal,
    Enterprise,
    Unknown,
}

/// A single hub entry with all display data.
#[derive(Debug, Clone)]
pub struct HubEntry {
    pub id: String,
    pub name: String,
    pub tier: HubTier,
    pub region: Option<String>,
}

impl HubEntry {
    /// Derive tier from APS extension_type string.
    pub fn tier_from_extension(ext: Option<&str>) -> HubTier {
        match ext {
            Some(e) if e.contains("autodesk.core:Hub") => HubTier::Personal,
            Some(e) if e.contains("autodesk.bim360:Account")
                || e.contains("autodesk.acc:Account")
                || e.contains("autodesk.accproject") => HubTier::Enterprise,
            _ => HubTier::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personal_hub_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.core:Hub")),
            HubTier::Personal
        );
    }

    #[test]
    fn test_bim360_enterprise_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.bim360:Account")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_acc_enterprise_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.acc:Account")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_accproject_enterprise_tier() {
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.accproject:Hub")),
            HubTier::Enterprise
        );
    }

    #[test]
    fn test_unknown_tier() {
        assert_eq!(HubEntry::tier_from_extension(None), HubTier::Unknown);
        assert_eq!(
            HubEntry::tier_from_extension(Some("hubs:autodesk.fusion:Hub")),
            HubTier::Unknown
        );
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cd /root/github/raps/raps
cargo test -p raps-cli context_banner 2>&1 | tail -20
```
Expected: compile error — module not yet wired.

**Step 3: Wire the module**

Add to `raps-cli/src/main.rs` line 38 (after `mod shell;`):
```rust
mod context_banner;
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p raps-cli context_banner 2>&1 | tail -20
```
Expected: `test context_banner::tests::test_personal_hub_tier ... ok` (5 tests pass)

**Step 5: Commit**

```bash
cd /root/github/raps/raps
git add raps-cli/src/context_banner.rs raps-cli/src/main.rs
git commit -m "feat: add context_banner module with HubTier classification"
```

---

### Task 2: Add `ContextBanner::from_hubs()` and `print_inline()`

**Files:**
- Modify: `raps-cli/src/context_banner.rs`

**Step 1: Write the failing tests**

Add to the `tests` module in `context_banner.rs`:

```rust
    #[test]
    fn test_from_hubs_classifies_tiers() {
        use raps_dm::types::{Hub, HubAttributes, HubExtension};
        let hubs = vec![
            Hub {
                hub_type: "hubs".into(),
                id: "a.abc123".into(),
                attributes: HubAttributes {
                    name: "My Projects".into(),
                    region: Some("US".into()),
                    extension: Some(HubExtension {
                        extension_type: Some("hubs:autodesk.core:Hub".into()),
                    }),
                },
            },
            Hub {
                hub_type: "hubs".into(),
                id: "b.01fb1602-2ec0-4b05-bf6e-39dc70b3ae05".into(),
                attributes: HubAttributes {
                    name: "Acme Corp".into(),
                    region: Some("US".into()),
                    extension: Some(HubExtension {
                        extension_type: Some("hubs:autodesk.bim360:Account".into()),
                    }),
                },
            },
        ];
        let banner = ContextBanner::from_hubs(&hubs);
        assert_eq!(banner.hubs.len(), 2);
        assert_eq!(banner.hubs[0].tier, HubTier::Personal);
        assert_eq!(banner.hubs[1].tier, HubTier::Enterprise);
    }

    #[test]
    fn test_short_id_truncation() {
        let entry = HubEntry {
            id: "b.01fb1602-2ec0-4b05-bf6e-39dc70b3ae05".into(),
            name: "Acme Corp".into(),
            tier: HubTier::Enterprise,
            region: Some("US".into()),
        };
        let short = entry.short_id();
        assert!(short.len() <= 16);
        assert!(short.contains("…") || short.len() < 16);
    }
```

**Step 2: Implement `ContextBanner` and `print_inline()`**

Add above the `#[cfg(test)]` block in `context_banner.rs`:

```rust
use colored::Colorize;
use raps_dm::types::Hub;

/// Collection of hub entries for display.
pub struct ContextBanner {
    pub hubs: Vec<HubEntry>,
}

impl HubEntry {
    /// Shortened ID for inline display (max 16 chars).
    pub fn short_id(&self) -> String {
        if self.id.len() <= 16 {
            self.id.clone()
        } else {
            // Keep prefix (up to 8 chars) + ellipsis + last 4 chars
            let prefix = &self.id[..8.min(self.id.len())];
            let suffix = &self.id[self.id.len().saturating_sub(4)..];
            format!("{}…{}", prefix, suffix)
        }
    }

    /// Glyph + label string for this tier.
    pub fn tier_label(&self) -> &'static str {
        match self.tier {
            HubTier::Personal   => "○ PERSONAL  ",
            HubTier::Enterprise => "◆ ENTERPRISE",
            HubTier::Unknown    => "? UNKNOWN   ",
        }
    }
}

impl ContextBanner {
    /// Build banner from a slice of DM Hub objects.
    pub fn from_hubs(hubs: &[Hub]) -> Self {
        let entries = hubs
            .iter()
            .map(|h| {
                let ext = h.attributes.extension.as_ref()
                    .and_then(|e| e.extension_type.as_deref());
                HubEntry {
                    id: h.id.clone(),
                    name: h.attributes.name.clone(),
                    tier: HubEntry::tier_from_extension(ext),
                    region: h.attributes.region.clone(),
                }
            })
            .collect();
        Self { hubs: entries }
    }

    /// Build banner from a single resolved enterprise account (for admin commands).
    pub fn from_account(id: &str, name: &str, region: Option<&str>) -> Self {
        Self {
            hubs: vec![HubEntry {
                id: id.to_string(),
                name: name.to_string(),
                tier: HubTier::Enterprise,
                region: region.map(|s| s.to_string()),
            }],
        }
    }

    /// Print one inline line per hub to stderr.
    /// Format (80-col safe):
    ///   ○ PERSONAL    My Projects              a.aBcD…xyz  [US]
    ///   ◆ ENTERPRISE  Acme Corp                01fb…ae05   [US]
    ///
    /// Only prints in interactive (table) mode — caller checks OutputFormat.
    pub fn print_inline(&self) {
        for entry in &self.hubs {
            let region = entry.region.as_deref().unwrap_or("--");
            let name_col = format!("{:<28}", truncate(&entry.name, 28));
            let id_col   = format!("{:<14}", entry.short_id());
            let line = format!(
                "  {}  {}  {}  [{}]",
                entry.tier_label(),
                name_col,
                id_col,
                region
            );
            match entry.tier {
                HubTier::Personal   => eprintln!("{}", line.dimmed()),
                HubTier::Enterprise => eprintln!("{}", line.cyan().bold()),
                HubTier::Unknown    => eprintln!("{}", line.dimmed()),
            }
        }
    }
}

/// Truncate a string to max_len chars, appending … if cut.
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}
```

**Step 3: Run tests**

```bash
cargo test -p raps-cli context_banner 2>&1 | tail -20
```
Expected: all 7 tests pass.

**Step 4: Commit**

```bash
git add raps-cli/src/context_banner.rs
git commit -m "feat: add ContextBanner::from_hubs and print_inline"
```

---

### Task 3: Add `print_box()` for admin context

**Files:**
- Modify: `raps-cli/src/context_banner.rs`

**Step 1: Write the failing test**

Add to tests module:

```rust
    #[test]
    fn test_box_line_width() {
        // Each rendered box line must be exactly BOX_WIDTH visible chars.
        // We test the helper directly.
        let line = box_line("Account ID:   01fb1602-2ec0-4b05-bf6e-39dc70b3ae05");
        // Strip ANSI for length check
        let visible: String = strip_ansi(&line);
        assert_eq!(visible.chars().count(), BOX_WIDTH,
            "Box line visible width should be {BOX_WIDTH}, got {}", visible.chars().count());
    }
```

Add helper at bottom of file (outside tests, before `#[cfg(test)]`):

```rust
/// Strip ANSI escape codes for length measurement (test helper, also used internally).
#[allow(dead_code)]
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' { in_escape = true; continue; }
        if in_escape { if c == 'm' { in_escape = false; } continue; }
        out.push(c);
    }
    out
}
```

**Step 2: Implement `box_line` helper and `print_box()`**

Add to `context_banner.rs` (before `#[cfg(test)]`):

```rust
/// Render one content line inside the box borders.
/// Format: │  {content padded to INNER_WIDTH}│
/// Total visible width = BOX_WIDTH (80).
fn box_line(content: &str) -> String {
    let padded = format!("  {:<width$}", truncate(content, INNER_WIDTH - 2), width = INNER_WIDTH - 2);
    format!("│{}│", padded)
}

/// Render the top border: ┌─ {title} ─…─┐  (total BOX_WIDTH chars)
fn box_top(title: &str) -> String {
    let prefix = format!("┌─ {} ", title);
    let prefix_len = prefix.chars().count();
    let dashes = "─".repeat(BOX_WIDTH.saturating_sub(prefix_len + 1));
    format!("{}{}┐", prefix, dashes)
}

/// Render the bottom border (total BOX_WIDTH chars).
fn box_bottom() -> String {
    format!("└{}┘", "─".repeat(BOX_WIDTH - 2))
}
```

Add method to `ContextBanner` impl:

```rust
    /// Print a bordered admin context box to stderr.
    /// Shows the first enterprise hub entry (the resolved account).
    /// Only prints in interactive mode — caller checks OutputFormat.
    ///
    /// Example output (80 cols):
    /// ┌─ Account Context ────────────────────────────────────────────────────────────┐
    /// │  ◆ ENTERPRISE  Acme Corp                                                     │
    /// │  Account ID:   01fb1602-2ec0-4b05-bf6e-39dc70b3ae05                         │
    /// │  Region:       US                                                            │
    /// └──────────────────────────────────────────────────────────────────────────────┘
    pub fn print_box(&self) {
        let Some(entry) = self.hubs.iter().find(|h| h.tier == HubTier::Enterprise) else {
            return;
        };
        let region = entry.region.as_deref().unwrap_or("--");
        let top    = box_top("Account Context");
        let line1  = box_line(&format!("◆ ENTERPRISE  {}", entry.name));
        let line2  = box_line(&format!("Account ID:   {}", entry.id));
        let line3  = box_line(&format!("Region:       {}", region));
        let bottom = box_bottom();

        eprintln!("{}", top.cyan());
        eprintln!("{}", line1.cyan().bold());
        eprintln!("{}", line2.cyan());
        eprintln!("{}", line3.cyan());
        eprintln!("{}", bottom.cyan());
    }
```

**Step 3: Run tests**

```bash
cargo test -p raps-cli context_banner 2>&1 | tail -20
```
Expected: all tests pass including `test_box_line_width`.

**Step 4: Commit**

```bash
git add raps-cli/src/context_banner.rs
git commit -m "feat: add print_box for admin context display"
```

---

### Task 4: Add `print_warning_no_enterprise()`

**Files:**
- Modify: `raps-cli/src/context_banner.rs`

**Step 1: Write test**

Add to tests:

```rust
    #[test]
    fn test_warning_box_top_width() {
        let top = box_top("⚠  Enterprise Account Required");
        let visible = strip_ansi(&top);
        assert_eq!(visible.chars().count(), BOX_WIDTH);
    }
```

**Step 2: Add `print_warning_no_enterprise()` as a free function**

Add to `context_banner.rs` (inside `impl ContextBanner` or as a free function — use free function since it doesn't need Self):

```rust
/// Print a yellow warning box to stderr when no enterprise hub is found.
/// Called by resolve_account_id when 0 enterprise hubs exist.
pub fn print_warning_no_enterprise() {
    let top    = box_top("⚠  Enterprise Account Required");
    let line1  = box_line("Admin API is not available for personal hubs.");
    let line2  = box_line("Register your app in ACC Custom Integrations to");
    let line3  = box_line("enable admin commands for your enterprise account.");
    let line4  = box_line("Docs: rapscli.xyz/docs/custom-integrations");
    let bottom = box_bottom();

    eprintln!("{}", top.yellow().bold());
    eprintln!("{}", line1.yellow());
    eprintln!("{}", line2.yellow());
    eprintln!("{}", line3.yellow());
    eprintln!("{}", line4.yellow());
    eprintln!("{}", bottom.yellow().bold());
}
```

**Step 3: Run tests**

```bash
cargo test -p raps-cli context_banner 2>&1 | tail -20
```
Expected: all tests pass.

**Step 4: Commit**

```bash
git add raps-cli/src/context_banner.rs
git commit -m "feat: add print_warning_no_enterprise for missing enterprise hub"
```

---

### Task 5: Wire inline banner into `raps hub list`

**Files:**
- Modify: `raps-cli/src/commands/hub.rs` (around line 52–121)

**Step 1: Find the exact location**

In `hub.rs`, find `async fn list_hubs`. The hub data is fetched and then the table is printed. The hubs slice is available before the `match output_format` block.

**Step 2: Add the banner call**

In `list_hubs`, after fetching hubs and before the `match output_format` block, add:

```rust
    // Print context banner in table mode only
    if matches!(output_format, crate::output::OutputFormat::Table) {
        let banner = crate::context_banner::ContextBanner::from_hubs(&hubs);
        banner.print_inline();
        eprintln!(); // blank line before table
    }
```

Locate the exact insertion point: after `let hubs = client.list_hubs().await?;` (or equivalent fetch call), before `if hubs.is_empty()` check.

**Step 3: Build to verify no compile errors**

```bash
cargo build -p raps-cli 2>&1 | grep -E "^error" | head -20
```
Expected: no errors.

**Step 4: Manual smoke test**

```bash
raps hub list
```
Expected: inline tier lines appear above the hub table, dimmed for personal, cyan bold for enterprise.

**Step 5: Commit**

```bash
git add raps-cli/src/commands/hub.rs
git commit -m "feat: show inline account context banner in raps hub list"
```

---

### Task 6: Wire box + warning into `resolve_account_id`

**Files:**
- Modify: `raps-cli/src/commands/admin/mod.rs` (lines 494–554)

**Step 1: Replace the "no enterprise" error message**

Find this block (around line 526–533):
```rust
        if enterprise_hubs.is_empty() {
            anyhow::bail!(
                "No enterprise ACC/BIM360 accounts found.\n\
                 Admin commands require an enterprise Autodesk Construction Cloud or BIM360 account.\n\
                 ...\n\
                 Use --account <id> if you know your enterprise account ID."
            );
        }
```

Replace with:
```rust
        if enterprise_hubs.is_empty() {
            crate::context_banner::print_warning_no_enterprise();
            anyhow::bail!(
                "No enterprise ACC/BIM360 accounts found. \
                 Use --account <id> if you know your enterprise account ID."
            );
        }
```

**Step 2: Replace the auto-select stderr message**

Find this block (around lines 534–539):
```rust
        if enterprise_hubs.len() == 1 {
            let hub = &enterprise_hubs[0];
            let id = hub.id.trim_start_matches("b.").to_string();
            eprintln!("Using account: {} ({})", hub.attributes.name, id);
            return Ok(id);
        }
```

Replace with:
```rust
        if enterprise_hubs.len() == 1 {
            let hub = &enterprise_hubs[0];
            let id = hub.id.trim_start_matches("b.").to_string();
            let region = hub.attributes.region.as_deref();
            let banner = crate::context_banner::ContextBanner::from_account(
                &id, &hub.attributes.name, region,
            );
            banner.print_box();
            return Ok(id);
        }
```

**Step 3: Build and test**

```bash
cargo build -p raps-cli 2>&1 | grep "^error" | head -10
```
Expected: clean build.

**Step 4: Commit**

```bash
git add raps-cli/src/commands/admin/mod.rs
git commit -m "feat: show account context box and warning in admin commands"
```

---

### Task 7: Create `commands/status.rs` — full dashboard

**Files:**
- Create: `raps-cli/src/commands/status.rs`
- Modify: `raps-cli/src/commands/mod.rs`

**Step 1: Write the test first**

Create `raps-cli/src/commands/status.rs` with test:

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps status` — full context dashboard command.

use anyhow::Result;
use colored::Colorize;
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;

use crate::context_banner::{BOX_WIDTH, ContextBanner, HubTier};
use crate::output::OutputFormat;

pub async fn run_status(
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
    output_format: OutputFormat,
) -> Result<()> {
    if !matches!(output_format, OutputFormat::Table) {
        // For JSON/CSV output, print a simple status object
        print_status_structured(auth_client, dm_client).await?;
        return Ok(());
    }

    print_dashboard(auth_client, dm_client).await
}

async fn print_dashboard(
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
) -> Result<()> {
    let rule = "═".repeat(BOX_WIDTH);

    // Header
    eprintln!("{}", rule.bold());
    eprintln!("  {}", "RAPS Status".bold());
    eprintln!("{}", rule.bold());
    eprintln!();

    // ── Auth section ──────────────────────────────────────────────────────────
    eprintln!("  {}", section_rule("Auth"));

    let two_legged_ok = auth_client.test_auth().await.is_ok();
    let two_label = if two_legged_ok {
        "✓ Available".green().bold().to_string()
    } else {
        "✗ Not configured".red().to_string()
    };
    eprintln!("  {:<12}{:<22}(client credentials)", "2-legged".bold(), two_label);

    let logged_in = auth_client.is_logged_in().await;
    if logged_in {
        let token_info = auth_client.get_stored_token().ok().flatten();
        let expiry_str = if let Some(tok) = &token_info {
            let secs = tok.expires_at - chrono::Utc::now().timestamp();
            if secs > 0 {
                let h = secs / 3600;
                let m = (secs % 3600) / 60;
                if h > 0 {
                    format!("expires in {}h {}m", h, m).green().to_string()
                } else {
                    format!("expires in {}m", m).green().to_string()
                }
            } else {
                "Expired".red().bold().to_string()
            }
        } else {
            "unknown expiry".dimmed().to_string()
        };
        eprintln!("  {:<12}{:<22}{}", "3-legged".bold(), "✓ Logged in".green().bold(), expiry_str);
    } else {
        eprintln!("  {:<12}{}", "3-legged".bold(), "✗ Not logged in".red());
    }

    // Profile info
    if let Ok(config) = raps_kernel::config::Config::from_env_lenient() {
        let profile_name = std::env::var("RAPS_PROFILE")
            .or_else(|_| {
                // Read active profile name from profiles.json
                active_profile_name()
            })
            .unwrap_or_else(|_| "(default)".to_string());

        let client_id_display = if config.client_id.is_empty() {
            "not configured".red().to_string()
        } else {
            let id = &config.client_id;
            let masked = if id.len() > 8 {
                format!("{}…{}", &id[..4], &id[id.len()-4..])
            } else {
                id.clone()
            };
            masked.dimmed().to_string()
        };
        eprintln!("  {:<12}{:<22}client_id: {}", "Profile".bold(), profile_name.cyan(), client_id_display);
    }
    eprintln!();

    // ── Hubs section ──────────────────────────────────────────────────────────
    eprintln!("  {}", section_rule("Hubs"));

    match dm_client.list_hubs().await {
        Ok(hubs) if hubs.is_empty() => {
            eprintln!("  {}", "(no hubs found — check login and app permissions)".yellow());
        }
        Ok(hubs) => {
            let banner = ContextBanner::from_hubs(&hubs);
            for entry in &banner.hubs {
                let region = entry.region.as_deref().unwrap_or("--");
                let name_col = format!("{:<28}", crate::context_banner::truncate_pub(&entry.name, 28));
                let id_col   = format!("{:<20}", entry.short_id());
                let line = format!(
                    "  {}  {}  {}  [{}]",
                    entry.tier_label(), name_col, id_col, region
                );
                match entry.tier {
                    HubTier::Personal   => eprintln!("{}", line.dimmed()),
                    HubTier::Enterprise => {
                        eprintln!("{}", line.cyan().bold());
                        // Sub-info: account ID for enterprise hubs
                        let bare_id = entry.id.trim_start_matches("b.");
                        eprintln!("  {}  {}  {}", " ".repeat(12), "└─ Admin API: ✓ ready  Account ID:".dimmed(), bare_id.dimmed());
                    }
                    HubTier::Unknown    => eprintln!("{}", line.dimmed()),
                }
            }
        }
        Err(e) => {
            eprintln!("  {} {}", "Could not fetch hubs:".yellow(), e);
            eprintln!("  {}", "Run 'raps auth login' to authenticate.".dimmed());
        }
    }
    eprintln!();

    // ── Context section ────────────────────────────────────────────────────────
    eprintln!("  {}", section_rule("Context"));

    let ctx_keys = [
        ("account_id", "APS_ACCOUNT_ID"),
        ("hub_id",     "APS_HUB_ID"),
        ("project_id", "APS_PROJECT_ID"),
    ];
    for (label, env_key) in &ctx_keys {
        let (value, source) = match std::env::var(env_key) {
            Ok(v) => (v, format!("env:{}", env_key)),
            Err(_) => ("(not set)".to_string(), String::new()),
        };
        if source.is_empty() {
            eprintln!("  {:<14}{}", label.bold(), value.dimmed());
        } else {
            eprintln!("  {:<14}{:<46}  {}", label.bold(), value.cyan(), source.dimmed());
        }
    }

    eprintln!();
    eprintln!("{}", rule.bold());

    Ok(())
}

async fn print_status_structured(
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
) -> Result<()> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct StatusOut {
        two_legged_available: bool,
        three_legged_logged_in: bool,
        hubs: Vec<HubOut>,
    }
    #[derive(Serialize)]
    struct HubOut {
        id: String,
        name: String,
        tier: String,
        region: Option<String>,
    }

    let two = auth_client.test_auth().await.is_ok();
    let three = auth_client.is_logged_in().await;
    let hub_list = dm_client.list_hubs().await.unwrap_or_default();
    let banner = ContextBanner::from_hubs(&hub_list);

    let out = StatusOut {
        two_legged_available: two,
        three_legged_logged_in: three,
        hubs: banner.hubs.iter().map(|h| HubOut {
            id: h.id.clone(),
            name: h.name.clone(),
            tier: format!("{:?}", h.tier),
            region: h.region.clone(),
        }).collect(),
    };

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Render a section divider: "  Auth ───────────────────────────────────────"
fn section_rule(title: &str) -> String {
    let prefix = format!("{} ", title);
    let dashes = "─".repeat(BOX_WIDTH.saturating_sub(prefix.len() + 2));
    format!("{}{}", prefix.bold(), dashes.dimmed())
}

/// Read the active profile name from profiles.json without full config loading.
fn active_profile_name() -> std::result::Result<String, ()> {
    let dirs = directories::ProjectDirs::from("com", "autodesk", "raps").ok_or(())?;
    let path = dirs.config_dir().join("profiles.json");
    let content = std::fs::read_to_string(path).map_err(|_| ())?;
    let v: serde_json::Value = serde_json::from_str(&content).map_err(|_| ())?;
    v["active_profile"].as_str().map(|s| s.to_string()).ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_rule_fits_80_cols() {
        let rule = section_rule("Auth");
        // strip bold/dim ANSI
        let visible: String = rule.chars()
            .scan(false, |esc, c| {
                if c == '\x1b' { *esc = true; Some(None) }
                else if *esc { if c == 'm' { *esc = false; } Some(None) }
                else { Some(Some(c)) }
            })
            .flatten()
            .collect();
        assert!(visible.chars().count() <= BOX_WIDTH,
            "section_rule visible width {} > {BOX_WIDTH}", visible.chars().count());
    }
}
```

**Step 2: Expose `truncate` as `pub(crate)` in context_banner.rs**

In `context_banner.rs`, change `fn truncate` to `pub(crate) fn truncate_pub`:

Find:
```rust
/// Truncate a string to max_len chars, appending … if cut.
fn truncate(s: &str, max_len: usize) -> String {
```

Add an alias:
```rust
/// Public version of truncate for use in other modules.
pub(crate) fn truncate_pub(s: &str, max_len: usize) -> String {
    truncate(s, max_len)
}
```

**Step 3: Wire into `commands/mod.rs`**

Add `pub mod status;` after the `pub mod schema;` line in `raps-cli/src/commands/mod.rs`.

**Step 4: Build to verify**

```bash
cargo build -p raps-cli 2>&1 | grep "^error" | head -20
```

Fix any import issues. Common ones:
- Missing `use serde::Serialize;` — add it
- Missing `chrono` — check Cargo.toml for raps-cli; if not present, add `chrono = { workspace = true }` to raps-cli's `[dependencies]`
- `auth_client.get_stored_token()` — check actual method name in `raps-kernel/src/auth/` and adjust

**Step 5: Run tests**

```bash
cargo test -p raps-cli status 2>&1 | tail -10
```
Expected: `test_section_rule_fits_80_cols ... ok`

**Step 6: Commit**

```bash
git add raps-cli/src/commands/status.rs raps-cli/src/commands/mod.rs raps-cli/src/context_banner.rs
git commit -m "feat: add raps status dashboard command"
```

---

### Task 8: Wire `raps status` into main.rs

**Files:**
- Modify: `raps-cli/src/main.rs`

**Step 1: Add import**

In the `use commands::{...}` block at line 51, no change needed — `status` is a module, not a re-exported type.

**Step 2: Add `Status` variant to `Commands` enum**

After the `Doctor` variant (line 336), add:

```rust
    /// Show full context: auth, hubs (Personal vs Enterprise), active account
    Status,
```

**Step 3: Add to `command_name()`**

In the `command_name` match (around line 967), add before `Commands::External`:
```rust
        Commands::Status => "status",
```

**Step 4: Add to `execute_command()` dispatch**

In the `match command` block (around line 1075), add near `Commands::Hub`:

```rust
        Commands::Status => {
            commands::status::run_status(
                &get_auth_client(),
                &get_dm_client(),
                output_format,
            )
            .await?;
        }
```

**Step 5: Update `GROUPED_COMMANDS_HELP`**

In the `3-Legged Auth` section (line 91), add `status` entry:

```
  status        Full context: auth state, hub tiers (Personal/Enterprise), active account
```

**Step 6: Build and smoke test**

```bash
cargo build -p raps-cli 2>&1 | grep "^error" | head -10
raps status
```
Expected: full dashboard renders with auth, hubs, context sections.

**Step 7: Run full test suite**

```bash
cargo test --workspace 2>&1 | tail -20
cargo clippy --workspace 2>&1 | grep "^error" | head -10
```
Expected: no errors, no clippy errors.

**Step 8: Commit**

```bash
git add raps-cli/src/main.rs
git commit -m "feat: wire raps status into top-level command dispatch"
```

---

### Task 9: Final integration verification

**Step 1: Test all three surfaces**

```bash
# 1. Hub list — inline banner
raps hub list

# 2. Admin command — context box
raps admin user add test@example.com --dry-run

# 3. Status dashboard
raps status

# 4. Structured output (no banners)
raps status --output json
raps hub list --output json
```

**Step 2: Verify 80-col constraint**

```bash
# Run in 80-col terminal or pipe through cut to check
raps status 2>&1 | awk '{ if (length > 80) print NR": "length": "$0 }'
```
Expected: no lines longer than 80 chars.

**Step 3: Run clippy**

```bash
cargo clippy --workspace -- -D warnings 2>&1 | grep "^error" | head -20
```

**Step 4: Final commit if any fixes applied**

```bash
git add -p
git commit -m "fix: clippy and integration fixes for context banner"
```
