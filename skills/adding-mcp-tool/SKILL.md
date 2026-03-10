---
name: adding-mcp-tool
version: "1.0"
description: Use when adding a new MCP tool to the RAPS MCP server — covers tool schema definition, handler implementation, dispatch registration, and the client accessor pattern.
---

# Adding an MCP Tool

Add a new MCP tool to the RAPS server (101+ tools and growing).

**Repo:** `/root/github/raps/raps`
**Location:** `raps-cli/src/mcp/`

## Key Files

| File | Purpose |
|------|---------|
| `definitions.rs` | Tool schema (name, description, parameters) |
| `dispatch.rs` | Routes tool name to handler method |
| `tools_oss.rs` | OSS tool handlers (buckets, objects) |
| `tools_dm.rs` | Data Management handlers (hubs, projects, folders) |
| `tools_admin.rs` | Account Admin handlers (users, projects) |
| `tools_acc.rs` | ACC handlers (issues, RFIs, submittals, checklists) |
| `tools_misc.rs` | Auth, translate, webhooks, DA, reality, pipeline |
| `server.rs` | RapsServer struct, client accessors |

## Step 1: Define Schema (definitions.rs)

```rust
Tool::new(
    "foo_list",
    "List all foos. Returns name, status, and creation date.",
    schema(
        json!({
            "limit": {
                "type": "integer",
                "description": "Maximum number of results (default: 100)"
            },
            "status": {
                "type": "string",
                "description": "Filter by status: active, archived"
            }
        }),
        &[],  // required parameters (empty = all optional)
    ),
),
```

The `schema()` helper builds a JSON Schema object with type, properties, and required array.

## Step 2: Implement Handler (tools_*.rs)

```rust
impl RapsServer {
    pub(crate) async fn foo_list(
        &self,
        limit: Option<usize>,
        status: Option<String>,
    ) -> String {
        let client = self.get_foo_client().await;
        let limit = Self::clamp_limit(limit, 100, 500);

        match client.list_foos().await {
            Ok(foos) => {
                let filtered: Vec<_> = foos.iter()
                    .filter(|f| status.as_ref().map_or(true, |s| &f.status == s))
                    .take(limit)
                    .collect();

                let mut output = format!("Found {} foo(s):\n\n", filtered.len());
                for f in &filtered {
                    output.push_str(&format!("* {} (status: {})\n", f.name, f.status));
                }
                output
            }
            Err(e) => format!("Error listing foos: {}", e),
        }
    }
}
```

Key patterns:
- Return `String` (formatted for AI consumption, not JSON)
- Use `Self::clamp_limit()` to cap result counts
- Handle errors gracefully — return error message string, don't panic
- Get client via `self.get_*_client().await` (lazy-initialized, cached)

## Step 3: Register in Dispatch (dispatch.rs)

Add match arm in the tool dispatch:

```rust
"foo_list" => {
    let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);
    let status = args.get("status").and_then(|v| v.as_str()).map(String::from);
    self.foo_list(limit, status).await
}
```

Argument extraction patterns:
- String: `args.get("key").and_then(|v| v.as_str()).map(String::from)`
- Integer: `args.get("key").and_then(|v| v.as_u64()).map(|v| v as usize)`
- Boolean: `args.get("key").and_then(|v| v.as_bool()).unwrap_or(false)`
- Required: same but with `.unwrap_or_default()` or error handling

## Step 4: Add Client Accessor (if new domain)

Only needed if the domain crate doesn't already have a client accessor in `server.rs`:

```rust
pub(crate) async fn get_foo_client(&self) -> FooClient {
    if let Some(client) = self.foo_client.read().await.as_ref() {
        return client.clone();
    }
    let mut guard = self.foo_client.write().await;
    if guard.is_none() {
        *guard = Some(FooClient::new_with_http_config(
            (*self.config).clone(),
            self.http_config.clone(),
        ));
    }
    guard.as_ref().expect("initialized above").clone()
}
```

## Checklist

1. Add `Tool::new(...)` in `definitions.rs`
2. Implement handler method in appropriate `tools_*.rs`
3. Add dispatch match arm in `dispatch.rs`
4. Run `cargo check -p raps-cli`
5. Test manually: `raps serve` -> call tool via MCP client
