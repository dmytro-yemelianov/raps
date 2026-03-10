---
name: adding-raps-cli-command
version: "1.0"
description: Use when adding a new CLI command or subcommand to RAPS — covers the clap struct pattern, module wiring, output formatting, and test structure across the 10-crate workspace.
---

# Adding a RAPS CLI Command

Add a new command or subcommand following the established patterns.

**Repo:** `/root/github/raps/raps`

## Architecture

```
raps-kernel/   — config, auth, security, logging (shared by all)
raps-oss/      — OSS bucket/object API client
raps-derivative/ — Model Derivative API client
raps-dm/       — Data Management API client
raps-da/       — Design Automation API client
raps-acc/      — ACC Issues, RFIs, Submittals, Assets, Checklists
raps-webhooks/ — Webhooks API client
raps-reality/  — Reality Capture API client
raps-admin/    — Account Admin API client
raps-cli/      — CLI layer (clap, MCP server, commands)
```

Commands live in `raps-cli/src/commands/`. API clients live in domain crates.

## Command Pattern

### 1. Clap Subcommand Enum

```rust
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum FooCommands {
    /// List all foos
    List,
    /// Get foo details
    Info {
        foo_id: String,
    },
    /// Create a new foo
    Create {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "default")]
        policy: String,
    },
}
```

### 2. Execute Method

```rust
impl FooCommands {
    pub async fn execute(
        self,
        client: &FooClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            FooCommands::List => list_foos(client, output_format).await,
            FooCommands::Info { foo_id } => get_foo(client, &foo_id, output_format).await,
            FooCommands::Create { name, policy } => {
                create_foo(client, &name, &policy, output_format).await
            }
        }
    }
}
```

### 3. Output Struct

```rust
#[derive(Debug, Serialize)]
struct FooOutput {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}
```

### 4. Output Formatting

```rust
match output_format {
    OutputFormat::Table => {
        println!("{}", "Foo Details".bold());
        println!("{}", "-".repeat(60));
        println!("  {} {}", "Name:".bold(), output.name.cyan());
    }
    _ => {
        output_format.write(&output)?;
    }
}
```

## Wiring a New Command

1. Create `raps-cli/src/commands/foo.rs`
2. Add `pub mod foo;` to `raps-cli/src/commands/mod.rs`
3. Add variant to main CLI enum in `raps-cli/src/main.rs`:
   ```rust
   #[command(subcommand)]
   Foo(commands::foo::FooCommands),
   ```
4. Add match arm in dispatch:
   ```rust
   Commands::Foo(cmd) => cmd.execute(&foo_client, output_format).await,
   ```

## If Adding an API Client

1. Create function in the appropriate domain crate (e.g., `raps-oss/src/objects.rs`)
2. Use `reqwest` with the shared `HttpClientConfig`
3. Return domain types with `#[derive(Debug, Deserialize, Serialize)]`
4. Handle errors with `anyhow::Result`

## Test Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foo_output_serialization() {
        let output = FooOutput { id: "123".into(), name: "test".into(), description: None };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"id\":\"123\""));
    }
}
```

## Checklist

1. Create command file with Subcommand enum + execute method
2. Wire into mod.rs and main.rs
3. Create Serialize output struct
4. Handle Table + structured output formats
5. Add tests
6. Run `cargo clippy --workspace` and `cargo test --workspace`
