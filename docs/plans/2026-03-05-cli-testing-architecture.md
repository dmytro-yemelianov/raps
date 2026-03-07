# CLI Testing Architecture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the scenario-driven CLI testing system described in `raps_cli_testing_architecture.md`, covering API call tracing, scenario tests, CLI snapshot tests, operation tests, and edge cases.

**Architecture:** Extend `raps-mock`'s `TestServer` with a `TraceRecorder` that captures every HTTP write call made against it. Scenario tests instantiate real Rust API clients pointed at a local `TestServer`, run a workflow, then assert on the recorded API trace using `insta` snapshots. CLI command-tree tests use `assert_cmd` against the compiled binary.

**Tech Stack:** Rust 1.88, `raps-mock::TestServer` (axum 0.7), `insta` snapshots, `assert_cmd`, `raps-acc` clients (`AccountAdminClient`, `ProjectUsersClient`), `raps-kernel::test_utils`

---

## Context: What Already Exists

Before touching any code, understand the current state:

- `raps-cli/tests/*.rs` — flat list of 40+ files; no subdirectories except `snapshots/` (MCP auth only)
- `raps-admin/tests/integration/` — bulk executor tests (no HTTP, no CLI binary)
- `raps-acc/tests/project_users_role_test.rs` — the only existing `TestServer`-based test for a client
- **`raps-cli` does NOT have `raps-mock` as a dev-dependency** — must be added
- `raps-mock` is at `../raps-mock` (outside workspace), version `0.2.0`
- All admin clients (`AccountAdminClient`, `ProjectUsersClient`) use `get_3leg_token()` — 3-legged auth required for all admin API calls
- Mock server validates Bearer tokens via `StateManager::auth.validate_token()` — arbitrary strings are rejected
- Pattern for injecting valid token: POST to mock's `/authentication/v2/token`, then `auth.set_3leg_token_for_testing(token).await`
- Seeded mock data: account `mock-account-001`, projects `proj-001` (active), `proj-002` (active), project user `alice@example.com` in `proj-001` with `role-admin`

---

## Task 1: Add raps-mock dev-dependency to raps-cli

**Files:**
- Modify: `raps-cli/Cargo.toml`

**Step 1: Add the dependency**

In `raps-cli/Cargo.toml`, add to `[dev-dependencies]`:

```toml
raps-mock.workspace = true
```

**Step 2: Verify it compiles**

```bash
cargo build -p raps-cli --tests 2>&1 | grep -E "^error"
```
Expected: no output (clean build)

**Step 3: Commit**

```bash
git add raps-cli/Cargo.toml
git commit -m "chore(test): add raps-mock as dev-dep to raps-cli"
```

---

## Task 2: Add TraceRecorder to raps-mock

**Files:**
- Create: `/root/github/raps/raps-mock/src/trace.rs`
- Modify: `/root/github/raps/raps-mock/src/lib.rs`

**Step 1: Write the failing test**

Add to the bottom of `/root/github/raps/raps-mock/src/trace.rs` (create new file):

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! API call trace recorder for use in integration tests.

use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

/// A single recorded API call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiCall {
    pub method: String,
    pub path: String,
}

/// Thread-safe recorder that captures API calls made against the mock server.
/// Clone it freely — all clones share the same underlying list.
#[derive(Debug, Clone, Default)]
pub struct TraceRecorder {
    calls: Arc<Mutex<Vec<ApiCall>>>,
}

impl TraceRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one API call. Called by middleware on every write request.
    pub fn record(&self, call: ApiCall) {
        self.calls.lock().unwrap().push(call);
    }

    /// Return a snapshot of all calls recorded so far.
    pub fn calls(&self) -> Vec<ApiCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Clear the recorded calls (useful between test phases).
    pub fn clear(&self) {
        self.calls.lock().unwrap().clear();
    }

    /// Assert exactly `count` calls have been recorded.
    pub fn assert_call_count(&self, count: usize) {
        let calls = self.calls();
        assert_eq!(
            calls.len(),
            count,
            "Expected {count} calls, got {}.\nCalls: {calls:#?}",
            calls.len()
        );
    }

    /// Assert at least one call was made to `method` on a path containing `path_fragment`.
    pub fn assert_called_with(&self, method: &str, path_fragment: &str) {
        let calls = self.calls();
        assert!(
            calls.iter().any(|c| {
                c.method.eq_ignore_ascii_case(method) && c.path.contains(path_fragment)
            }),
            "Expected {method} {path_fragment} — not found.\nCalls: {calls:#?}"
        );
    }

    /// Assert NO call was made to `method` on a path containing `path_fragment`.
    pub fn assert_not_called_with(&self, method: &str, path_fragment: &str) {
        let calls = self.calls();
        assert!(
            !calls.iter().any(|c| {
                c.method.eq_ignore_ascii_case(method) && c.path.contains(path_fragment)
            }),
            "Expected {method} {path_fragment} NOT to be called.\nCalls: {calls:#?}"
        );
    }

    /// Return all POST calls whose path contains `path_fragment`.
    pub fn post_calls_to(&self, path_fragment: &str) -> Vec<ApiCall> {
        self.calls()
            .into_iter()
            .filter(|c| c.method.eq_ignore_ascii_case("POST") && c.path.contains(path_fragment))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_query() {
        let rec = TraceRecorder::new();
        rec.record(ApiCall { method: "POST".into(), path: "/projects/p1/users".into() });
        rec.record(ApiCall { method: "GET".into(), path: "/projects/p1/users/u1".into() });

        rec.assert_call_count(2);
        rec.assert_called_with("POST", "/projects/p1/users");
        rec.assert_called_with("GET", "/users/u1");
    }

    #[test]
    fn test_assert_not_called() {
        let rec = TraceRecorder::new();
        rec.assert_not_called_with("DELETE", "/projects");
    }

    #[test]
    fn test_post_calls_to() {
        let rec = TraceRecorder::new();
        rec.record(ApiCall { method: "POST".into(), path: "/projects/p1/users".into() });
        rec.record(ApiCall { method: "POST".into(), path: "/projects/p2/users".into() });
        rec.record(ApiCall { method: "PATCH".into(), path: "/projects/p1/users/u1".into() });

        assert_eq!(rec.post_calls_to("/users").len(), 2);
        assert_eq!(rec.post_calls_to("p2").len(), 1);
    }

    #[test]
    fn test_clear() {
        let rec = TraceRecorder::new();
        rec.record(ApiCall { method: "POST".into(), path: "/foo".into() });
        rec.clear();
        rec.assert_call_count(0);
    }

    #[test]
    fn test_clone_shares_state() {
        let rec = TraceRecorder::new();
        let rec2 = rec.clone();
        rec2.record(ApiCall { method: "DELETE".into(), path: "/bar".into() });
        rec.assert_call_count(1);
    }
}
```

**Step 2: Run failing test**

```bash
cargo test -p raps-mock trace 2>&1 | tail -5
```
Expected: `error[E0433]: failed to resolve: use of undeclared crate or module` (file not exported yet)

**Step 3: Export from lib.rs**

Add to `/root/github/raps/raps-mock/src/lib.rs` after the existing `pub mod` declarations:

```rust
pub mod trace;
pub use trace::TraceRecorder;
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p raps-mock trace 2>&1 | tail -10
```
Expected: `test result: ok. 5 passed`

**Step 5: Commit**

```bash
cd /root/github/raps/raps-mock && git add src/trace.rs src/lib.rs
git commit -m "feat(trace): add TraceRecorder for API call capture in tests"
cd /root/github/raps/raps
```

---

## Task 3: Add TestServer::start_with_trace()

**Files:**
- Modify: `/root/github/raps/raps-mock/src/testing.rs`

**Context:** `TestServer::start()` builds the axum app from `MockServer::router()`. We need a variant that wraps the router in an additional middleware layer that feeds every write request into a `TraceRecorder`.

The key insight: axum middleware added **after** `.layer()` runs **outermost** (first to see requests). In axum 0.7, `from_fn_with_state` passes state to a middleware fn.

**Step 1: Write the failing test (at the bottom of testing.rs)**

```rust
#[cfg(test)]
mod trace_tests {
    use super::*;

    #[tokio::test]
    async fn test_server_with_trace_records_post_calls() {
        let ts = TestServer::start_with_trace().await.unwrap();
        let client = reqwest::Client::new();

        // Auth call (not recorded — GET is skipped)
        let resp = client
            .post(format!("{}/authentication/v2/token", ts.url))
            .json(&serde_json::json!({
                "client_id": "test-client",
                "client_secret": "test-secret",
                "grant_type": "client_credentials"
            }))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let token = body["access_token"].as_str().unwrap().to_string();

        // Make a POST that should be recorded
        client
            .post(format!(
                "{}/construction/admin/v1/projects/proj-001/users",
                ts.url
            ))
            .bearer_auth(&token)
            .json(&serde_json::json!({"email": "trace@test.com", "roleId": "role-admin"}))
            .send()
            .await
            .unwrap();

        ts.trace.assert_call_count(1);
        ts.trace.assert_called_with("POST", "/projects/proj-001/users");
    }
}
```

**Step 2: Run to verify it fails**

```bash
cargo test -p raps-mock test_server_with_trace 2>&1 | tail -8
```
Expected: compile error — `TestServer` has no `start_with_trace` or `.trace` field.

**Step 3: Implement**

Add to `/root/github/raps/raps-mock/src/testing.rs` after the existing `TestServer` impl block:

```rust
use crate::trace::{ApiCall, TraceRecorder};
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// Variant of TestServer that also records every write API call.
pub struct TestServerWithTrace {
    /// Base URL of the running server.
    pub url: String,
    /// Shared trace recorder — query this after running operations.
    pub trace: TraceRecorder,
    _task: tokio::task::JoinHandle<()>,
}

impl TestServerWithTrace {
    /// Get the base URL of the server.
    pub fn uri(&self) -> &str {
        &self.url
    }
}

impl Drop for TestServerWithTrace {
    fn drop(&mut self) {
        self._task.abort();
    }
}

/// Axum middleware that records write operations into a TraceRecorder.
async fn recording_middleware(
    axum::Extension(recorder): axum::Extension<TraceRecorder>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    // Record POST, PATCH, PUT, DELETE — skip GET and auth endpoints
    if matches!(method.as_str(), "POST" | "PATCH" | "PUT" | "DELETE")
        && path != "/authentication/v2/token"
    {
        recorder.record(ApiCall { method, path });
    }
    next.run(request).await
}

impl TestServer {
    /// Start a server that also records all write-API calls.
    /// Access recorded calls via `TestServerWithTrace::trace`.
    pub async fn start_with_trace() -> Result<TestServerWithTrace> {
        let config = MockServerConfig::default();
        let server = MockServer::new(config).await?;

        let recorder = TraceRecorder::new();
        let app = server
            .router()
            .layer(axum::Extension(recorder.clone()))
            .layer(axum::middleware::from_fn(recording_middleware));

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let task = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("Trace test server failed: {}", e);
            }
        });

        Ok(TestServerWithTrace {
            url: format!("http://{}", addr),
            trace: recorder,
            _task: task,
        })
    }
}
```

**Step 4: Run test**

```bash
cargo test -p raps-mock test_server_with_trace 2>&1 | tail -8
```
Expected: `test result: ok. 1 passed`

**Step 5: Commit**

```bash
cd /root/github/raps/raps-mock && git add src/testing.rs
git commit -m "feat(testing): add TestServer::start_with_trace() for scenario tests"
cd /root/github/raps/raps
```

---

## Task 4: Write TEST_AUDIT.md

**Files:**
- Create: `TEST_AUDIT.md` (workspace root)

Create a concise audit document. Content (write the file):

```markdown
# Test Audit

Generated: 2026-03-05

## Existing test files

### raps-cli/tests/ (CLI integration tests, assert_cmd)
- admin_commands.rs — help text and arg-parsing tests for admin subcommands
- auth_commands.rs, bucket_commands.rs, ... — command-level tests for each domain
- live_api_tests.rs — real APS API tests (#[ignore], require APS_CLIENT_ID)
- mcp_auth_tests.rs — MCP tool availability tests with insta snapshots

### raps-admin/tests/integration/
- add_user_tests.rs — BulkExecutor unit tests (mock closures, no HTTP)
- dry_run_tests.rs, resume_tests.rs, remove_user_tests.rs, update_role_tests.rs, folder_rights_tests.rs

### raps-acc/tests/
- project_users_role_test.rs — HTTP round-trip tests using TestServer

### raps-kernel/src/auth/tests.rs — AuthClient unit tests with TestServer

## What is covered

| Layer | Covered |
|-------|---------|
| CLI arg parsing / help text | Partial (help text only, no full workflow) |
| Bulk executor behavior | Yes (raps-admin/tests) |
| API client serialization | Partial (role_id None/Some) |
| HTTP round-trip for add_user | Yes (project_users_role_test.rs) |
| HTTP round-trip for remove_user | No |
| HTTP round-trip for archive_project | No |
| API call trace / sequence verification | No |
| Scenario tests (full workflow) | No |
| CLI help structure snapshot | No |
| Dry-run produces zero writes | Partial (BulkExecutor level only) |
| Edge cases (invalid role, project archived) | No |

## Missing test layers

1. TraceRecorder in raps-mock (infrastructure)
2. tests/operations/ — per-operation HTTP round-trip tests
3. tests/scenarios/ — full workflow tests with API trace assertion
4. tests/cli/help_structure.rs — snapshot of command tree
5. tests/snapshots/api_traces/ — golden API call sequences
6. Edge cases: user not member, project archived, invalid role, rate limit
7. TEST_COVERAGE.md (to be generated after implementation)
```

**No test to run for this task.** Commit the file:

```bash
git add TEST_AUDIT.md
git commit -m "docs(test): add TEST_AUDIT.md from architecture gap analysis"
```

---

## Task 5: Create directory structure and runner files

**Files:**
- Create: `raps-cli/tests/operations.rs`
- Create: `raps-cli/tests/operations/mod.rs`
- Create: `raps-cli/tests/scenarios.rs`
- Create: `raps-cli/tests/scenarios/mod.rs`
- Create: `raps-cli/tests/cli/mod.rs`
- Create: `raps-cli/tests/cli_tests.rs`
- Create: `raps-cli/tests/test_utils/mod.rs`

**Important Rust rule:** Files directly in `tests/` are separate test binaries. Files in `tests/subdir/` are modules, included via `mod subdir;` in a runner file.

**Step 1: Create the runner files**

`raps-cli/tests/operations.rs`:
```rust
//! Operation-level tests: one HTTP round-trip per admin operation.
//! Each test uses TestServer and asserts API call behavior.

mod test_utils;
mod operations;
```

`raps-cli/tests/operations/mod.rs`:
```rust
pub mod add_user_to_project;
pub mod remove_user_from_project;
pub mod add_user_to_all_projects;
pub mod archive_project;
```

`raps-cli/tests/scenarios.rs`:
```rust
//! Scenario tests: full admin workflows with API trace verification.

mod test_utils;
mod scenarios;
```

`raps-cli/tests/scenarios/mod.rs`:
```rust
pub mod admin_add_user_all_projects;
pub mod admin_remove_user;
pub mod admin_archive_project;
pub mod admin_dry_run;
```

`raps-cli/tests/cli_tests.rs`:
```rust
//! CLI command tree and output format tests using assert_cmd + insta.

mod cli;
```

`raps-cli/tests/cli/mod.rs`:
```rust
pub mod help_structure;
```

`raps-cli/tests/test_utils/mod.rs` (shared helpers, included by both operations.rs and scenarios.rs):
```rust
//! Shared test utilities for operation and scenario tests.

use raps_acc::admin::AccountAdminClient;
use raps_acc::users::ProjectUsersClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;
use raps_kernel::types::StoredToken;
use raps_mock::TestServer;

pub struct TestClients {
    pub admin: AccountAdminClient,
    pub users: std::sync::Arc<ProjectUsersClient>,
    pub auth: AuthClient,
}

pub fn make_clients(base_url: &str) -> TestClients {
    let config = Config {
        client_id: "test-client".into(),
        client_secret: "test-secret".into(),
        base_url: base_url.to_string(),
        callback_url: "http://localhost:8080/callback".into(),
        da_nickname: None,
        http_config: HttpClientConfig::default(),
    };
    let auth = AuthClient::new(config.clone());
    TestClients {
        admin: AccountAdminClient::new_with_http_config(
            config.clone(),
            auth.clone(),
            HttpClientConfig::default(),
        ),
        users: std::sync::Arc::new(ProjectUsersClient::new(config, auth.clone())),
        auth,
    }
}

/// Get a valid token from the mock server and inject it as the 3-leg token.
pub async fn inject_token(auth: &AuthClient, base_url: &str) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/authentication/v2/token", base_url))
        .json(&serde_json::json!({
            "client_id": "test-client",
            "client_secret": "test-secret",
            "grant_type": "client_credentials"
        }))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let access_token = body["access_token"].as_str().unwrap().to_string();
    auth.set_3leg_token_for_testing(StoredToken {
        access_token,
        refresh_token: None,
        expires_at: chrono::Utc::now().timestamp() + 3600,
        scopes: vec![],
    })
    .await;
}
```

**Step 2: Create placeholder stubs** (so the runner files compile)

`raps-cli/tests/operations/add_user_to_project.rs`:
```rust
// placeholder — tests added in Task 6
```
Same for the other three operation files and three scenario files and `cli/help_structure.rs`.

**Step 3: Verify it compiles**

```bash
cargo test -p raps-cli --test operations 2>&1 | tail -5
cargo test -p raps-cli --test scenarios 2>&1 | tail -5
cargo test -p raps-cli --test cli_tests 2>&1 | tail -5
```
Expected: `running 0 tests` / `test result: ok. 0 passed`

**Step 4: Commit**

```bash
git add raps-cli/tests/operations.rs raps-cli/tests/operations/ \
        raps-cli/tests/scenarios.rs raps-cli/tests/scenarios/ \
        raps-cli/tests/cli_tests.rs raps-cli/tests/cli/ \
        raps-cli/tests/test_utils/
git commit -m "test: add scenario/operation/cli test directory structure"
```

---

## Task 6: Operation test — add_user_to_project

**Files:**
- Modify: `raps-cli/tests/operations/add_user_to_project.rs`

This tests `ProjectUsersClient::add_user` at the HTTP level: correct endpoint, correct role forwarding, duplicate handling.

**Step 1: Write the tests**

```rust
//! Operation tests: adding a single user to a single project.

use raps_acc::users::AddProjectUserRequest;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_add_user_sends_post_to_correct_endpoint() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    clients
        .users
        .add_user(
            "proj-001",
            AddProjectUserRequest {
                email: "new@example.com".into(),
                role_id: None,
                products: vec![],
            },
        )
        .await
        .unwrap();

    server
        .trace
        .assert_called_with("POST", "/projects/proj-001/users");
    server.trace.assert_call_count(1);
}

#[tokio::test]
async fn test_add_user_with_role_id_sends_role_in_body() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .users
        .add_user(
            "proj-001",
            AddProjectUserRequest {
                email: "roletest@example.com".into(),
                role_id: Some("role-project-admin".into()),
                products: vec![],
            },
        )
        .await
        .unwrap();

    assert_eq!(result.role_id, Some("role-project-admin".into()));
}

#[tokio::test]
async fn test_add_user_without_role_omits_role_id_key() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .users
        .add_user(
            "proj-001",
            AddProjectUserRequest {
                email: "norole@example.com".into(),
                role_id: None,
                products: vec![],
            },
        )
        .await
        .unwrap();

    // Mock assigns "role-default" when no roleId in body
    assert_eq!(result.role_id, Some("role-default".into()));
}
```

**Step 2: Run**

```bash
cargo test -p raps-cli --test operations add_user_to_project 2>&1 | tail -10
```
Expected: `test result: ok. 3 passed`

**Step 3: Commit**

```bash
git add raps-cli/tests/operations/add_user_to_project.rs
git commit -m "test(ops): add_user_to_project operation tests"
```

---

## Task 7: Operation test — remove_user_from_project

**Files:**
- Modify: `raps-cli/tests/operations/remove_user_from_project.rs`

```rust
//! Operation tests: removing a user from a single project.

use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_remove_existing_user_sends_delete() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // user-001 / alice@example.com is seeded in proj-001
    clients.users.remove_user("proj-001", "user-001").await.unwrap();

    server
        .trace
        .assert_called_with("DELETE", "/projects/proj-001/users/user-001");
    server.trace.assert_call_count(1);
}

#[tokio::test]
async fn test_remove_nonexistent_user_returns_error() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients.users.remove_user("proj-001", "user-does-not-exist").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("404"));
}
```

**Step 2: Run**

```bash
cargo test -p raps-cli --test operations remove_user 2>&1 | tail -10
```
Expected: `test result: ok. 2 passed`

**Step 3: Commit**

```bash
git add raps-cli/tests/operations/remove_user_from_project.rs
git commit -m "test(ops): remove_user_from_project operation tests"
```

---

## Task 8: Operation test — add_user_to_all_projects

**Files:**
- Modify: `raps-cli/tests/operations/add_user_to_all_projects.rs`

This is the core operation. It tests `raps_admin::bulk_add_user` end-to-end against the mock server.

```rust
//! Operation tests: bulk_add_user across all active projects.

use std::sync::Arc;

use raps_admin::{BulkConfig, bulk_add_user};
use raps_admin::filter::ProjectFilter;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_bulk_add_user_calls_post_for_each_active_project() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // Mock has proj-001 and proj-002 both active
    let result = bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "bulk@example.com",
        None,
        &ProjectFilter::default(),
        BulkConfig { concurrency: 2, dry_run: false, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.total, 2);
    assert_eq!(result.completed, 2);
    assert_eq!(result.failed, 0);

    // One POST per project
    assert_eq!(server.trace.post_calls_to("/users").len(), 2);
}

#[tokio::test]
async fn test_bulk_add_user_with_role_id_sends_role_to_each_project() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "roletest@example.com",
        Some("role-project-admin"),
        &ProjectFilter::default(),
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.completed, 2);
    // Both projects received a POST
    server.trace.assert_called_with("POST", "/projects/proj-001/users");
    server.trace.assert_called_with("POST", "/projects/proj-002/users");
}

#[tokio::test]
async fn test_bulk_add_user_dry_run_makes_no_post_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "dryrun@example.com",
        None,
        &ProjectFilter::default(),
        BulkConfig { dry_run: true, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.skipped, result.total);
    assert_eq!(result.completed, 0);
    // Dry-run must not make any write calls to the API
    server.trace.assert_call_count(0);
}
```

**Step 2: Run**

```bash
cargo test -p raps-cli --test operations add_user_to_all 2>&1 | tail -10
```
Expected: `test result: ok. 3 passed`

**Step 3: Commit**

```bash
git add raps-cli/tests/operations/add_user_to_all_projects.rs
git commit -m "test(ops): add_user_to_all_projects operation tests with trace verification"
```

---

## Task 9: Operation test — archive_project

**Files:**
- Modify: `raps-cli/tests/operations/archive_project.rs`

```rust
//! Operation tests: archiving a project via AccountAdminClient.

use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_archive_project_sends_patch_with_status_archived() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .admin
        .update_project(
            "mock-account-001",
            "proj-001",
            None,
            Some("archived".into()),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.status, "archived");
    server.trace.assert_called_with("PATCH", "/projects/proj-001");
}

#[tokio::test]
async fn test_archive_nonexistent_project_returns_error() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .admin
        .update_project("mock-account-001", "proj-does-not-exist", None, Some("archived".into()), None, None)
        .await;

    assert!(result.is_err());
}
```

**Note:** Check the exact signature of `AccountAdminClient::update_project` before writing. Run:

```bash
cargo doc -p raps-acc --no-deps --open 2>/dev/null || grep -n "pub async fn update_project" /root/github/raps/raps/raps-acc/src/admin/projects.rs
```

Adjust argument list to match the actual signature.

**Step 2: Run**

```bash
cargo test -p raps-cli --test operations archive_project 2>&1 | tail -10
```
Expected: `test result: ok. 2 passed`

**Step 3: Commit**

```bash
git add raps-cli/tests/operations/archive_project.rs
git commit -m "test(ops): archive_project operation tests"
```

---

## Task 10: Scenario test — admin_add_user_all_projects (with insta trace snapshot)

**Files:**
- Modify: `raps-cli/tests/scenarios/admin_add_user_all_projects.rs`

This is the primary scenario test. It runs the full workflow and compares the recorded API call sequence to a golden snapshot.

**Step 1: Write the test**

```rust
//! Scenario: admin adds a user to all active projects as project administrator.
//!
//! Expected API call sequence:
//!   1. GET /accounts/{account}/projects  (list projects — not a write, not in trace)
//!   2. GET /projects/{id}/users/{email}  (duplicate check — not a write, not in trace)
//!   3. POST /projects/proj-001/users     (add user)
//!   4. POST /projects/proj-002/users     (add user)

use std::sync::Arc;

use raps_admin::{BulkConfig, bulk_add_user};
use raps_admin::filter::ProjectFilter;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_add_user_to_all_projects_trace_matches_snapshot() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "scenario@example.com",
        Some("role-project-admin"),
        &ProjectFilter::default(),
        BulkConfig { concurrency: 1, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    let calls = server.trace.calls();
    // Normalize to method + path pairs for snapshot
    let trace: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| serde_json::json!({"method": c.method, "path": c.path}))
        .collect();

    insta::assert_json_snapshot!("admin_add_user_all_projects_trace", trace);
}

#[tokio::test]
async fn test_add_user_all_projects_result_counts() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "counts@example.com",
        None,
        &ProjectFilter::default(),
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.total, 2, "2 active projects in mock");
    assert!(result.failed == 0, "no failures expected");
}
```

**Step 2: Run (first run creates snapshot)**

```bash
cargo test -p raps-cli --test scenarios admin_add_user_all_projects 2>&1 | tail -15
```

On first run, `insta` creates a snapshot file under `raps-cli/tests/snapshots/`. Review it:

```bash
cat raps-cli/tests/snapshots/scenarios__admin_add_user_all_projects_trace.snap
```

Expected snapshot content (two POST calls to /users endpoints):
```
---
source: raps-cli/tests/scenarios/admin_add_user_all_projects.rs
snapshot_kind: json
---
[
  {
    "method": "POST",
    "path": "/construction/admin/v1/projects/proj-001/users"
  },
  {
    "method": "POST",
    "path": "/construction/admin/v1/projects/proj-002/users"
  }
]
```

If snapshot looks correct, accept it:

```bash
cargo insta accept 2>/dev/null || cargo insta review
```

**Step 3: Run again to verify snapshot passes**

```bash
cargo test -p raps-cli --test scenarios admin_add_user_all_projects 2>&1 | tail -5
```
Expected: `test result: ok. 2 passed`

**Step 4: Commit**

```bash
git add raps-cli/tests/scenarios/admin_add_user_all_projects.rs \
        raps-cli/tests/snapshots/scenarios__admin_add_user_all_projects_trace.snap
git commit -m "test(scenario): admin add user to all projects — trace snapshot"
```

---

## Task 11: Scenario test — admin_remove_user

**Files:**
- Modify: `raps-cli/tests/scenarios/admin_remove_user.rs`

```rust
//! Scenario: admin removes a user from all projects.

use std::sync::Arc;

use raps_admin::{BulkConfig, bulk_remove_user};
use raps_admin::filter::ProjectFilter;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_remove_user_trace_contains_delete_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // alice@example.com is seeded in proj-001 (user-001)
    bulk_remove_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "alice@example.com",
        &ProjectFilter::default(),
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    let calls = server.trace.calls();
    let trace: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| serde_json::json!({"method": c.method, "path": c.path}))
        .collect();

    insta::assert_json_snapshot!("admin_remove_user_trace", trace);
}

#[tokio::test]
async fn test_remove_user_dry_run_sends_no_delete() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    bulk_remove_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "alice@example.com",
        &ProjectFilter::default(),
        BulkConfig { dry_run: true, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    server.trace.assert_not_called_with("DELETE", "/users");
}
```

Same snapshot workflow as Task 10. Run, accept snapshot, commit.

---

## Task 12: Scenario test — admin_archive_project

**Files:**
- Modify: `raps-cli/tests/scenarios/admin_archive_project.rs`

```rust
//! Scenario: admin archives a project.

use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_archive_project_trace_snapshot() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    clients
        .admin
        .update_project("mock-account-001", "proj-002", None, Some("archived".into()), None, None)
        .await
        .unwrap();

    let calls = server.trace.calls();
    let trace: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| serde_json::json!({"method": c.method, "path": c.path}))
        .collect();

    insta::assert_json_snapshot!("admin_archive_project_trace", trace);
}
```

Run, accept snapshot, commit.

---

## Task 13: Scenario test — dry_run produces no writes

**Files:**
- Modify: `raps-cli/tests/scenarios/admin_dry_run.rs`

```rust
//! Scenario: --dry-run guarantees no write API calls are made for any operation.

use std::sync::Arc;

use raps_admin::{BulkConfig, bulk_add_user, bulk_remove_user};
use raps_admin::filter::ProjectFilter;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_dry_run_add_makes_zero_write_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "dryrun@example.com",
        Some("role-admin"),
        &ProjectFilter::default(),
        BulkConfig { dry_run: true, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    server.trace.assert_call_count(0);
}

#[tokio::test]
async fn test_dry_run_remove_makes_zero_write_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    bulk_remove_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "alice@example.com",
        &ProjectFilter::default(),
        BulkConfig { dry_run: true, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    server.trace.assert_call_count(0);
}
```

Run tests, commit.

---

## Task 14: CLI help structure snapshot

**Files:**
- Modify: `raps-cli/tests/cli/help_structure.rs`

```rust
//! Snapshot tests for the CLI command tree.
//! Catch accidental renames, removals, or flag changes.

use assert_cmd::Command;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_admin_help_snapshot() {
    let output = raps()
        .args(["admin", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_help", stdout);
}

#[test]
fn test_admin_user_help_snapshot() {
    let output = raps()
        .args(["admin", "user", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_user_help", stdout);
}

#[test]
fn test_admin_project_help_snapshot() {
    let output = raps()
        .args(["admin", "project", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_project_help", stdout);
}

#[test]
fn test_admin_user_add_to_all_projects_help_snapshot() {
    let output = raps()
        .args(["admin", "user", "add-to-all-projects", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_user_add_to_all_projects_help", stdout);
}
```

**Run, accept snapshots, commit:**

```bash
cargo test -p raps-cli --test cli_tests help_structure 2>&1 | tail -10
cargo insta accept
git add raps-cli/tests/cli/ raps-cli/tests/snapshots/
git commit -m "test(cli): add help structure snapshot tests for admin command tree"
```

---

## Task 15: Edge case tests

**Files:**
- Create: `raps-cli/tests/operations/edge_cases.rs`
- Modify: `raps-cli/tests/operations/mod.rs` (add `pub mod edge_cases;`)

```rust
//! Edge case tests for admin operations.

use raps_acc::users::AddProjectUserRequest;
use raps_admin::{BulkConfig, bulk_add_user};
use raps_admin::filter::ProjectFilter;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

/// user already exists in a project → should be counted as "skipped", not "failed"
#[tokio::test]
async fn test_add_user_already_member_is_skipped_not_failed() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // Add once (succeeds)
    clients
        .users
        .add_user("proj-001", AddProjectUserRequest {
            email: "dup@example.com".into(),
            role_id: None,
            products: vec![],
        })
        .await
        .unwrap();

    // Add again — same email, same project → duplicate check should skip
    let result = bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "dup@example.com",
        None,
        &ProjectFilter::default(),
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.failed, 0, "duplicates must not count as failures");
}

/// account has no active projects → result total is 0, no API calls made
#[tokio::test]
async fn test_no_active_projects_returns_empty_result() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // Filter to a project ID that doesn't exist
    let filter = ProjectFilter::from_ids(vec!["proj-nonexistent".into()]);

    let result = bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "empty@example.com",
        None,
        &filter,
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.total, 0);
    assert_eq!(result.completed, 0);
}
```

**Note:** Check `ProjectFilter` API — look at `raps-admin/src/filter.rs` for the exact constructor names before writing. Adjust `from_ids` to whatever the actual method is called.

**Run, commit.**

---

## Task 16: Write TEST_COVERAGE.md

**Files:**
- Create: `TEST_COVERAGE.md` (workspace root)

```markdown
# Test Coverage Summary

Generated: 2026-03-05

## Covered commands

| Command | Arg parsing | Operation test | Scenario test | Snapshot |
|---------|-------------|---------------|---------------|----------|
| admin user add-to-all-projects | ✓ | ✓ | ✓ | ✓ |
| admin user add | ✓ | ✓ | — | — |
| admin user remove | ✓ | ✓ | ✓ | ✓ |
| admin project archive | ✓ | ✓ | ✓ | ✓ |
| admin project list | ✓ | — | — | — |
| admin project create | ✓ | — | — | — |
| admin user update | ✓ | — | — | — |
| admin folder rights | ✓ | — | — | — |
| admin operation | ✓ | — | — | — |

## Coverage gaps (known)

- admin project create — no scenario test
- admin user update — no scenario test
- admin folder rights — no scenario test
- rate-limit (429) retry behavior — unit tested in raps-admin, not HTTP-level tested
- invalid role ID (role not found on server) — not tested
- project with suspended status — not tested

## How to run

```bash
# All tests (excluding live API)
cargo test --workspace

# Only scenario/operation tests
cargo test -p raps-cli --test operations
cargo test -p raps-cli --test scenarios

# CLI structure snapshots
cargo test -p raps-cli --test cli_tests

# Update snapshots after intentional changes
cargo insta review
```

## CI requirement

All tests in this list run with no network access. `raps-mock` is the only external dependency and runs in-process.
```

**Commit:**

```bash
git add TEST_COVERAGE.md
git commit -m "docs(test): add TEST_COVERAGE.md"
```

---

## Final verification

Run the full workspace test suite and confirm everything is green:

```bash
cargo test --workspace 2>&1 | grep -E "^(test result|FAILED|error\[)"
```

Expected: all `test result: ok. N passed`, zero `FAILED` lines.

Run clippy:

```bash
cargo clippy --workspace 2>&1 | grep "^error"
```

Expected: no output.

---

## Summary of files changed

| File | Action |
|------|--------|
| `raps-cli/Cargo.toml` | Add `raps-mock` dev-dep |
| `/root/github/raps/raps-mock/src/trace.rs` | New: TraceRecorder |
| `/root/github/raps/raps-mock/src/lib.rs` | Export trace module |
| `/root/github/raps/raps-mock/src/testing.rs` | Add `start_with_trace()` + `TestServerWithTrace` |
| `raps-cli/tests/operations.rs` | New runner |
| `raps-cli/tests/operations/{4 files}` | New operation tests |
| `raps-cli/tests/scenarios.rs` | New runner |
| `raps-cli/tests/scenarios/{4 files}` | New scenario tests |
| `raps-cli/tests/cli_tests.rs` | New runner |
| `raps-cli/tests/cli/help_structure.rs` | New CLI tree snapshot tests |
| `raps-cli/tests/test_utils/mod.rs` | New shared helpers |
| `raps-cli/tests/snapshots/*.snap` | Insta snapshot files |
| `TEST_AUDIT.md` | New |
| `TEST_COVERAGE.md` | New |
