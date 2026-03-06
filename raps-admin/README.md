# Developing `raps admin` Operations

This guide provides complete, detailed, and tested instructions on how to work on, extend, and test the bulk account administration logic in the `raps-admin` crate.

## Architecture Overview

The `raps-admin` module is specifically designed to handle bulk, long-running operations across potentially thousands of projects. To do this safely, it implements a structured, state-driven workflow instead of making direct inline API calls.

1. **Operations (`src/operations/`)**: Business logic defining how to execute a specific task on a single project (e.g., `AddUser`, `UpdateRole`). Must implement the `AdminOperation` trait.
2. **Executor (`src/bulk/executor.rs`)**: A parallelizing runner that takes a collection of projects and an `AdminOperation`, distributing the work across a thread pool while handling API rate limiting, automatic backoff, and retries.
3. **State Management (`src/bulk/state.rs`)**: Operations are stateful. Results (success, failure, skipped) are tracked and can be saved to disk, allowing interrupted operations to be resumed later without duplicating work.
4. **CLI Layer (`raps-cli/src/commands/admin/`)**: The presentation layer that parses user inputs, initializes the `raps-admin` operation, spins up the executor, and draws the progress bars/summary tables.

## How to Add a New Admin Command

### Step 1: Implement the Operation in `raps-admin`
1. Create a new file in `raps-admin/src/operations/` (e.g., `archive_project.rs`).
2. Define a struct holding the required parameters (e.g., `pub struct ArchiveProject`).
3. Implement the `AdminOperation` trait for your struct:

```rust
use async_trait::async_trait;
use raps_acc::admin::AccountAdminClient;
use crate::bulk::types::{AdminOperation, OperationResult};

pub struct ArchiveProject;

#[async_trait]
impl AdminOperation for ArchiveProject {
    fn name(&self) -> &str {
        "ArchiveProject"
    }

    async fn execute(
        &self,
        client: &AccountAdminClient,
        account_id: &str,
        project_id: &str,
    ) -> Result<OperationResult, anyhow::Error> {
        // Implement API call logic
        client.archive_project(account_id, project_id).await?;
        Ok(OperationResult::Success(format!("Archived {}", project_id)))
    }

    fn is_retryable_error(&self, err: &anyhow::Error) -> bool {
        let err_msg = err.to_string().to_lowercase();
        err_msg.contains("timeout") || err_msg.contains("502") || err_msg.contains("503")
    }
}
```
4. Register the module in `raps-admin/src/operations/mod.rs`.

### Step 2: Implement the CLI Command in `raps-cli`
1. Add the subcommand to `raps-cli/src/commands/admin/mod.rs`.
2. Parse the command in `raps-cli/src/commands/admin/mod.rs` match statement.
3. Construct your operation struct and pass it to the bulk executor logic. Make sure to support the `--dry-run` and `--concurrency` flags, using `raps_admin::bulk::executor::execute_bulk_operation()`.

### Step 3: Write Integration Tests in `raps-admin`
Tests in `raps-admin/tests/` verify the logic isolated from CLI flags, ensuring bulk state is tracked correctly.

1. Add tests in `raps-admin/tests/integration_tests.rs`.
2. Use the `TestServer` from `raps-mock` to spin up a local APS server.
3. Assert that the operation processes the correct number of items and logs successes.

```rust
#[tokio::test]
async fn test_archive_projects_bulk() {
    let server = raps_mock::TestServer::start_default().await.unwrap();
    let client = setup_test_client(&server.url).await;
    
    // Setup projects...
    let operation = ArchiveProject;
    
    // Run execution
    let config = BulkConfig { concurrency: 2, dry_run: false, ..Default::default() };
    let summary = executor::execute_bulk_operation(&client, "acc-id", &projects, operation, config, |_|{}).await.unwrap();
    
    assert_eq!(summary.success, 2);
}
```

### Step 4: Write Scenario Tests in `raps-cli`
To verify the full end-to-end command line argument parsing, formatting, and exit codes:

1. Add a test in `raps-cli/tests/scenarios/admin_cli_scenarios.rs`.
2. Use `crate::test_utils::start_cli_test()` to initialize `assert_cmd` connected to `raps-mock`.

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_project_archive_cli() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "admin",
        "project",
        "archive",
        "--account", "mock-account-001",
        "--project", "proj-001"
    ])
    .assert()
    .success();
}
```

## Running the Tests

To ensure your code meets the quality standards and has high confidence, always run the full test suite targeting the admin components:

```bash
# 1. Run unit and isolated integration tests inside raps-admin
cargo test -p raps-admin

# 2. Run the end-to-end CLI scenarios linked to raps-mock
cargo test -p raps-cli --test scenarios

# 3. Ensure code coverage is maintained
cargo clippy -p raps-admin -- -D warnings
cargo fmt --manifest-path raps-admin/Cargo.toml -- --check
```

If you modify API payloads, remember to verify the output in `raps-mock` matches what is explicitly stated in the APS OpenAPI Specification.
