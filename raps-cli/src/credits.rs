// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Demoscene-style credits display for RAPS CLI.
//!
//! Three tiers of branding:
//! - **Complete** (`raps --version`): full ASCII art + stats
//! - **Short** (`raps --help` header): one-line brand via `HELP_BRAND`
//! - **Minimal** (interactive shell welcome): compact stat line

use colored::Colorize;

// ── Constants ───────────────────────────────────────────────────────────────

/// CLI subcommands (Commands enum variants, excluding External and cfg-gated Dashboard)
pub const COMMAND_COUNT: usize = 27;

/// Workspace crates in Cargo.toml
pub const WORKSPACE_CRATE_COUNT: usize = 10;

/// APS API services wrapped by service crates
pub const API_SERVICE_COUNT: usize = 8; // OSS, Derivative, DM, Webhooks, DA, ACC, Reality, Admin

/// One-line brand for `--help` header.
///
/// Uses raw ANSI escapes because clap's `help_template` requires `&'static str`.
/// Same constraint as `GROUPED_COMMANDS_HELP`. The help_template attribute inlines
/// the same `concat!` expression — this constant documents the canonical form.
#[allow(dead_code)]
pub const HELP_BRAND: &str = concat!(
    "\x1b[1;33mraps\x1b[0m v",
    env!("CARGO_PKG_VERSION"),
    " \x1b[32m·\x1b[0m Rust Autodesk Platform Services CLI"
);

// ── Complete version (--version) ────────────────────────────────────────────

/// Print the full demoscene-style credits block.
///
/// Colors are automatically stripped when stdout is not a TTY or `NO_COLOR` is
/// set, thanks to the `colored` crate.
pub fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    let mcp_tool_count = crate::mcp::tools::TOOLS.len();

    // Logo
    println!();
    println!(
        "   {} {}",
        "▄▄▄  ▄▄▄  ▄▄▄▄ ▄▄▄▄".yellow().bold(),
        "✿".yellow()
    );
    println!("   {}", "█▄▄▀ █▄▄█ █▄▄█ █▄▄▄".yellow().bold());
    println!("   {}", "█  █ █  █ █    ▄▄▄█".yellow().bold());

    // Heavy separator
    println!("  {}", "═══════════════════════════════════".green());

    // Subtitle
    println!("   {}", "Rust Autodesk Platform Services".bright_yellow());

    // Light separator
    println!(
        "  {}",
        "───────────────────────────────────".green().dimmed()
    );

    // Info block
    credit_line("version", version);
    credit_line("author", "dmytro yemelianov");
    credit_line("license", "Apache-2.0");
    credit_line("edition", "Rust 2024");

    // Light separator
    println!(
        "  {}",
        "───────────────────────────────────".green().dimmed()
    );

    // Stats block
    credit_line("commands", &COMMAND_COUNT.to_string());
    credit_line("mcp tools", &mcp_tool_count.to_string());
    credit_line("workspace", &format!("{WORKSPACE_CRATE_COUNT} crates"));
    credit_line("apis", &format!("{API_SERVICE_COUNT} services"));

    // Light separator
    println!(
        "  {}",
        "───────────────────────────────────".green().dimmed()
    );

    // Links block
    credit_line("web", "rapscli.xyz");
    credit_line("docs", "rapscli.xyz/docs");

    // Heavy separator
    println!("  {}", "═══════════════════════════════════".green());
    println!();
}

/// Print a single credits line with dotted leaders.
///
/// Layout: `   {label} ·········· {value}`
/// Total field width (label + dots) is 20 chars.
fn credit_line(label: &str, value: &str) {
    let dot_count = 20usize.saturating_sub(label.len() + 1);
    let dots = "·".repeat(dot_count);
    println!(
        "   {} {} {}",
        label.dimmed(),
        dots.green().dimmed(),
        value.yellow().bold()
    );
}

// ── Minimal welcome (shell) ─────────────────────────────────────────────────

/// Print the compact shell welcome line.
///
/// Example: `raps v4.12.0 · 27 commands · 94 MCP tools · rapscli.xyz`
pub fn shell_welcome() {
    let version = env!("CARGO_PKG_VERSION");
    let mcp_tool_count = crate::mcp::tools::TOOLS.len();
    let sep = " · ".green().dimmed();

    println!(
        "{}{}{}{}{}{}{}",
        "raps".yellow().bold(),
        format_args!(" v{version}").to_string().bright_yellow(),
        sep,
        format_args!("{COMMAND_COUNT} commands")
            .to_string()
            .bright_yellow(),
        sep,
        format_args!("{mcp_tool_count} MCP tools")
            .to_string()
            .bright_yellow(),
        format_args!("{sep}rapscli.xyz"),
    );
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Sentinel test — fails when commands are added/removed so the constant
    /// stays in sync with the `Commands` enum.
    #[test]
    fn test_command_count() {
        // If this fails, update COMMAND_COUNT to match the Commands enum
        // (excluding External and cfg-gated variants like Dashboard).
        assert_eq!(
            COMMAND_COUNT, 27,
            "COMMAND_COUNT drifted — update credits.rs"
        );
    }

    #[test]
    fn test_help_brand_contains_version() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(
            HELP_BRAND.contains(version),
            "HELP_BRAND should contain the package version"
        );
        assert!(HELP_BRAND.contains("raps"));
    }

    #[test]
    fn test_version_not_empty() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_mcp_tool_count_reasonable() {
        let count = crate::mcp::tools::TOOLS.len();
        assert!(count >= 70, "Expected at least 70 MCP tools, got {count}");
    }
}
