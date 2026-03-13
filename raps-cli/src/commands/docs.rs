// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps docs` — generate agent-facing documentation from live code.

use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DocsCommands {
    /// Generate AGENTS.md — MCP tool reference for AI assistants
    Mcp {
        /// Exit non-zero if AGENTS.md exists but is out of date
        #[arg(long)]
        check: bool,

        /// Write output directly to AGENTS.md instead of stdout
        #[arg(long)]
        write: bool,
    },
}

impl DocsCommands {
    pub fn execute(self) -> Result<()> {
        match self {
            DocsCommands::Mcp { check, write } => generate_mcp_docs(check, write),
        }
    }
}

fn generate_mcp_docs(check: bool, write: bool) -> Result<()> {
    let content = build_agents_md();

    if write {
        let path = std::path::Path::new("AGENTS.md");
        std::fs::write(path, &content)?;
        println!("Written {} ({} bytes)", path.display(), content.len());
        return Ok(());
    }

    if check {
        let existing = std::fs::read_to_string("AGENTS.md").unwrap_or_default();
        if existing == content {
            println!("AGENTS.md is up to date.");
            return Ok(());
        } else {
            anyhow::bail!("AGENTS.md is stale. Run from repo root: raps docs mcp --write");
        }
    }

    print!("{}", content);
    Ok(())
}

fn build_agents_md() -> String {
    use crate::mcp::auth_guidance::{AuthRequirement, get_tool_auth_requirement};
    use crate::mcp::definitions::get_tools;

    let tools = get_tools();
    let mut md = String::new();

    md.push_str("# RAPS MCP Tool Reference\n\n");
    md.push_str("> Auto-generated from source — do not edit manually.\n");
    md.push_str("> Regenerate: `raps docs mcp --write`\n\n");
    md.push_str("This file describes every tool exposed by the RAPS MCP server.\n\n");

    md.push_str("## Authentication\n\n");
    md.push_str("| Type | When Required |\n");
    md.push_str("|---|---|\n");
    md.push_str("| 2-legged (client credentials) | OSS, Model Derivative, Admin bulk ops |\n");
    md.push_str(
        "| 3-legged (user authorization) | Data Management, ACC (issues, RFIs, assets) |\n\n",
    );
    md.push_str("Set `APS_CLIENT_ID` and `APS_CLIENT_SECRET` before starting the MCP server.\n");
    md.push_str("For 3-legged auth in headless environments: `raps auth login --device`\n\n");

    md.push_str("## Tools\n\n");
    md.push_str("| Tool | Auth | Description |\n");
    md.push_str("|---|---|---|\n");

    for tool in &tools {
        let auth_label = match get_tool_auth_requirement(tool.name.as_ref()) {
            AuthRequirement::TwoLegged => "2-leg",
            AuthRequirement::ThreeLegged => "3-leg",
            AuthRequirement::Either => "either",
        };
        let desc = tool.description.as_deref().unwrap_or("");
        md.push_str(&format!(
            "| `{}` | {} | {} |\n",
            tool.name, auth_label, desc
        ));
    }

    md.push('\n');

    md.push_str("## Output Schemas\n\n");
    md.push_str("All structured output types are queryable at runtime:\n\n");
    md.push_str("```\n");
    md.push_str("raps schema list                 # list available types\n");
    md.push_str("raps schema generate <name>      # JSON Schema for a specific type\n");
    md.push_str("raps schema all                  # all schemas as one JSON object\n");
    md.push_str("```\n\n");

    md.push_str("## Agent Invariants\n\n");
    md.push_str("Things that are true of RAPS CLI that agents cannot discover from `--help`:\n\n");
    md.push_str("- Non-interactive output defaults to JSON automatically (piped stdout → JSON, TTY → table)\n");
    md.push_str("- `RAPS_OUTPUT_FORMAT=json` forces JSON regardless of TTY\n");
    md.push_str(
        "- `--dry-run` is supported by all `admin` bulk operations and `pipeline` commands\n",
    );
    md.push_str("- All bucket keys must be globally unique, 3–128 chars, lowercase alphanumeric + hyphens\n");
    md.push_str("- Object URNs for translation must be Base64-encoded; get them via `raps object urn` or the `object_urn` MCP tool\n");
    md.push_str("- 3-legged tokens expire; use `auth_status` to check before long workflows\n");
    md.push_str(
        "- Admin bulk operations are resumable — if interrupted, use `admin_operation_resume`\n",
    );
    md.push_str("- `raps api` is a raw HTTP passthrough for API endpoints not yet covered by dedicated commands\n");

    md
}
