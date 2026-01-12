# Code Coverage Improvement Plan

## Current State Analysis

### Test Infrastructure
- **Test runner**: cargo-nextest for fast parallel test execution
- **Coverage tool**: cargo-llvm-cov with LCOV output
- **CI integration**: Codecov for coverage tracking
- **Total tests**: 166 tests across 21 files

### Current Test Distribution

| Location | Tests | Focus |
|----------|-------|-------|
| `tests/smoke_cli.rs` | 3 | CLI binary execution |
| `tests/integration_test.rs` | ~30 | CLI commands (mostly `#[ignore]`) |
| `tests/api_auth_mock_test.rs` | 2 | Auth client with mock server |
| `tests/command_dispatch_test.rs` | ~19 | Command routing |
| `tests/shell_test.rs` | ~1 | Shell functionality |
| `tests/enhanced_shell_test.rs` | ~2 | Shell extensions |
| `raps-kernel/src/error.rs` | 12 | Exit codes, error interpretation |
| `raps-kernel/src/config.rs` | 13 | Configuration loading |
| `raps-kernel/src/auth.rs` | 6 | Token management |
| `raps-kernel/src/storage.rs` | 1 | Token storage |
| `raps-kernel/src/progress.rs` | 5 | Progress bar utilities |
| `raps-kernel/src/prompts.rs` | ~5 | Prompt utilities |
| `raps-oss/src/lib.rs` | 6 | OSS types/enums |
| `raps-derivative/src/lib.rs` | 9 | Output formats |
| `raps-da/src/lib.rs` | 9 | DA types |
| `raps-dm/src/lib.rs` | 6 | DM types |
| `raps-acc/src/lib.rs` | 4 | ACC types |
| `raps-webhooks/src/lib.rs` | 6 | Webhook types |
| `raps-reality/src/lib.rs` | 9 | Reality types |
| `raps-cli/src/plugins.rs` | 14 | Plugin system |
| `raps-cli/src/shell.rs` | 3 | Shell completion |
| `raps-cli/src/commands/pipeline.rs` | 5 | Pipeline parsing |
| `raps-cli/src/commands/generate.rs` | 3 | Code generation |

### Coverage Gaps Identified

1. **HTTP layer** (`raps-kernel/src/http.rs`)
   - No tests for retry logic
   - No tests for timeout handling
   - No tests for client configuration

2. **Output formatting** (`raps-kernel/src/output.rs`)
   - No tests for JSON/YAML/CSV/Table output
   - No tests for color handling

3. **Interactive module** (`raps-kernel/src/interactive.rs`)
   - No tests for non-interactive mode detection
   - No tests for `--yes` flag handling

4. **API clients** (all `*-lib.rs` files)
   - Limited mock testing for API interactions
   - No tests for pagination handling
   - No tests for error responses

5. **Command handlers** (`raps-cli/src/commands/*.rs`)
   - Most commands untested
   - No integration with mock servers

6. **MCP server** (`raps-cli/src/mcp/`)
   - Not tested at all

---

## Implementation Plan

### Phase 1: Unit Test Foundation (Priority: High)

#### 1.1 HTTP Layer Tests (`raps-kernel/src/http.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_secs, 1);
    }

    #[test]
    fn test_http_config_custom() {
        let config = HttpClientConfig {
            timeout_secs: 60,
            max_retries: 5,
            retry_delay_secs: 2,
        };
        assert_eq!(config.timeout_secs, 60);
    }

    #[tokio::test]
    async fn test_retry_on_5xx() {
        // Use mock server to test retry behavior
    }

    #[tokio::test]
    async fn test_no_retry_on_4xx() {
        // 4xx errors should not retry
    }
}
```

#### 1.2 Output Format Tests (`raps-kernel/src/output.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestData {
        id: String,
        name: String,
    }

    #[test]
    fn test_json_output() {
        let data = TestData { id: "1".into(), name: "test".into() };
        let output = OutputFormat::Json;
        // Capture stdout and verify JSON format
    }

    #[test]
    fn test_yaml_output() {
        // Similar for YAML
    }

    #[test]
    fn test_csv_output() {
        // Similar for CSV
    }

    #[test]
    fn test_table_output() {
        // Verify table formatting
    }

    #[test]
    fn test_supports_colors() {
        assert!(OutputFormat::Table.supports_colors());
        assert!(!OutputFormat::Json.supports_colors());
    }
}
```

#### 1.3 Interactive Module Tests (`raps-kernel/src/interactive.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_interactive_env_var() {
        std::env::set_var("RAPS_NON_INTERACTIVE", "1");
        assert!(is_non_interactive());
        std::env::remove_var("RAPS_NON_INTERACTIVE");
    }

    #[test]
    fn test_yes_flag() {
        set_yes(true);
        assert!(is_yes());
        set_yes(false);
    }

    #[test]
    fn test_ci_detection() {
        std::env::set_var("CI", "true");
        assert!(is_non_interactive());
        std::env::remove_var("CI");
    }
}
```

### Phase 2: API Client Mock Tests (Priority: High)

#### 2.1 Shared Mock Server Infrastructure

Create `raps-kernel/src/test_utils.rs`:

```rust
//! Test utilities for mocking API responses

use std::net::TcpListener;
use std::thread;
use tiny_http::{Response, Server, StatusCode};

pub fn start_mock_server(status: u16, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let addr = listener.local_addr().expect("read addr");
    let server = Server::from_listener(listener, None).expect("start server");

    thread::spawn(move || {
        if let Some(request) = server.incoming_requests().next() {
            let response = Response::from_string(body).with_status_code(StatusCode(status));
            let _ = request.respond(response);
        }
    });

    format!("http://{}", addr)
}

pub fn start_multi_response_server(responses: Vec<(u16, &'static str)>) -> String {
    // For testing pagination, retries, etc.
}
```

#### 2.2 OSS Client Tests (`raps-oss/src/lib.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_bucket_success() {
        let server_url = start_mock_server(200, r#"{"bucketKey":"test-bucket"}"#);
        // Create client with mock URL and test
    }

    #[tokio::test]
    async fn test_create_bucket_conflict() {
        let server_url = start_mock_server(409, r#"{"reason":"Bucket already exists"}"#);
        // Verify proper error handling
    }

    #[tokio::test]
    async fn test_list_buckets_pagination() {
        // Test pagination handling
    }

    #[tokio::test]
    async fn test_upload_object_progress() {
        // Test progress callback
    }
}
```

#### 2.3 Derivative Client Tests (`raps-derivative/src/lib.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translate_success() {
        let server_url = start_mock_server(200, r#"{"result":"success","urn":"..."}"#);
        // Test translation start
    }

    #[tokio::test]
    async fn test_get_manifest() {
        // Test manifest retrieval
    }

    #[tokio::test]
    async fn test_translate_with_root_filename() {
        // Test ZIP file translation
    }
}
```

### Phase 3: Integration Tests (Priority: Medium)

#### 3.1 Enable Existing Integration Tests

Many tests in `tests/integration_test.rs` are marked `#[ignore]`. Enable tests that don't require credentials:

- `test_cli_help` - Remove ignore
- `test_cli_version` - Remove ignore
- `test_cli_invalid_command` - Remove ignore
- `test_output_format_*` - Remove ignore
- `test_*_help` - Remove ignore
- `test_completions_*` - Remove ignore

#### 3.2 Add Config Tests Without Credentials

```rust
#[test]
fn test_config_profile_create_delete() {
    // Create temp profile, verify, delete
}

#[test]
fn test_config_profile_switch() {
    // Create two profiles, switch between
}
```

### Phase 4: Command Handler Tests (Priority: Medium)

#### 4.1 Add Mock-Based Command Tests

For each command module in `raps-cli/src/commands/`:

```rust
// bucket.rs
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_list_buckets_empty() {
        // Mock empty response
    }

    #[tokio::test]
    async fn test_create_bucket_validates_name() {
        // Test bucket naming rules
    }
}
```

### Phase 5: Shell and Plugin Tests (Priority: Low)

#### 5.1 Shell Completion Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completer_top_level_commands() {
        let helper = RapsHelper::new();
        let completions = helper.complete_command("");
        assert!(completions.iter().any(|c| c.contains("bucket")));
        assert!(completions.iter().any(|c| c.contains("object")));
    }

    #[test]
    fn test_completer_subcommands() {
        let helper = RapsHelper::new();
        let completions = helper.complete_command("bucket ");
        assert!(completions.iter().any(|c| c.contains("list")));
    }
}
```

#### 5.2 Plugin System Tests

Already has 14 tests. Add:

```rust
#[test]
fn test_plugin_manifest_validation() {
    // Test invalid manifest handling
}

#[test]
fn test_plugin_version_compatibility() {
    // Test version range checking
}
```

---

## Coverage Targets

| Crate | Current | Target | Notes |
|-------|---------|--------|-------|
| raps-kernel | ~40% | 70% | Focus on http, output, interactive |
| raps-oss | ~30% | 60% | Add mock API tests |
| raps-derivative | ~35% | 60% | Add mock API tests |
| raps-dm | ~25% | 50% | Add mock API tests |
| raps-da | ~30% | 55% | Add mock API tests |
| raps-acc | ~20% | 45% | Add mock API tests |
| raps-webhooks | ~25% | 50% | Add mock API tests |
| raps-reality | ~30% | 50% | Add mock API tests |
| raps-cli | ~15% | 35% | Focus on commands, shell |

**Overall target**: 50% line coverage (up from estimated ~25%)

---

## Implementation Order

1. **Week 1**: Phase 1 (Unit tests for kernel modules)
   - http.rs tests
   - output.rs tests
   - interactive.rs tests

2. **Week 2**: Phase 2 (API client mocks)
   - Create test_utils.rs
   - OSS client tests
   - Derivative client tests

3. **Week 3**: Phase 3 (Integration tests)
   - Enable ignored tests
   - Add config tests

4. **Week 4**: Phase 4-5 (Commands and extras)
   - Command handler tests
   - Shell/plugin tests

---

## Test Utilities to Add

### 1. `raps-kernel/src/test_utils.rs`

Shared test infrastructure for:
- Mock HTTP server creation
- Test configuration builders
- Temp file/directory helpers
- Output capture utilities

### 2. Dev Dependencies to Add

```toml
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.2"
tiny_http = "0.12"
mockito = "1.2"  # Alternative to tiny_http
tokio-test = "0.4"
```

---

## Metrics and Monitoring

1. **Run coverage locally**:
   ```bash
   cargo llvm-cov nextest --all-features --workspace --html
   ```

2. **View report**: Open `target/llvm-cov/html/index.html`

3. **CI tracking**: Codecov dashboard shows trends over time

4. **Coverage gate**: Consider adding minimum coverage requirement to CI
