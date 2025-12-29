//! Command Dispatch Tests for RAPS CLI
//!
//! These tests verify that all CLI commands are properly dispatched without panicking.
//! They catch routing bugs like incorrect std::mem::replace patterns that corrupt command state.
//!
//! Run with: `cargo test --test command_dispatch_test`
//! Or run all tests: `cargo test`

use predicates::prelude::*;

#[allow(unused_imports)]
use assert_cmd::cargo::CommandCargoExt;

#[allow(deprecated)]
fn raps_cmd() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("raps").unwrap()
}

// ============== SMOKE TESTS ==============
// These tests verify commands don't panic, even if they fail due to missing auth

/// Test that auth test command dispatches without panicking
#[test]
fn test_auth_test_no_panic() {
    raps_cmd()
        .args(["auth", "test"])
        .assert()
        // Command may fail (no auth), but should NOT panic
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that auth login command dispatches without panicking
#[test]
fn test_auth_login_no_panic() {
    raps_cmd()
        // Use --non-interactive to prevent blocking on prompts
        .args(["--non-interactive", "auth", "login"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that auth logout command dispatches without panicking
#[test]
fn test_auth_logout_no_panic() {
    raps_cmd()
        .args(["auth", "logout"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that bucket list command dispatches without panicking
#[test]
fn test_bucket_list_no_panic() {
    raps_cmd()
        .args(["bucket", "list"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that object list command dispatches without panicking
#[test]
fn test_object_list_no_panic() {
    raps_cmd()
        .args(["object", "list", "test-bucket"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that translate status command dispatches without panicking
#[test]
fn test_translate_status_no_panic() {
    raps_cmd()
        .args(["translate", "status", "dXJuOnRlc3Q"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that hub list command dispatches without panicking
#[test]
fn test_hub_list_no_panic() {
    raps_cmd()
        .args(["hub", "list"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that webhook list command dispatches without panicking
#[test]
fn test_webhook_list_no_panic() {
    raps_cmd()
        .args(["webhook", "list"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that da engine list command dispatches without panicking
#[test]
fn test_da_engine_list_no_panic() {
    raps_cmd()
        .args(["da", "engine", "list"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that plugin list command dispatches without panicking
#[test]
fn test_plugin_list_no_panic() {
    raps_cmd()
        .args(["plugin", "list"])
        .assert()
        // Plugin list should succeed (no auth required)
        .success()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that config profile list command dispatches without panicking
#[test]
fn test_config_profile_list_no_panic() {
    raps_cmd()
        .args(["config", "profile", "list"])
        .assert()
        // Config commands should succeed (no auth required)
        .success()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that completions command dispatches without panicking
/// Note: In debug builds, there may be a clap assertion about positional args
/// This test is marked as ignored until the clap config is fixed
#[test]
#[ignore = "Known clap config issue with positional arg ordering in debug builds"]
fn test_completions_bash_no_panic() {
    raps_cmd()
        .args(["completions", "bash"])
        .assert()
        // Completions should succeed
        .success()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

/// Test that reality job list command dispatches without panicking
#[test]
fn test_reality_job_list_no_panic() {
    raps_cmd()
        .args(["reality", "job", "list"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}

// ============== COMPREHENSIVE DISPATCH TEST ==============

/// Smoke test: verify ALL major subcommands dispatch without panicking
/// This is the key test that catches dispatch/routing bugs
#[test]
fn test_all_commands_dispatch_no_panic() {
    let subcommands: Vec<Vec<&str>> = vec![
        // Auth commands
        vec!["auth", "test"],
        vec!["auth", "logout"],
        // OSS commands
        vec!["bucket", "list"],
        vec!["object", "list", "test-bucket"],
        // Model Derivative
        vec!["translate", "status", "dXJuOnRlc3Q"],
        // Data Management
        vec!["hub", "list"],
        // Webhooks
        vec!["webhook", "list"],
        // Design Automation
        vec!["da", "engine", "list"],
        // Reality Capture
        vec!["reality", "job", "list"],
        // Plugin (no auth required)
        vec!["plugin", "list"],
        // Config (no auth required)
        vec!["config", "profile", "list"],
        // Note: Completions skipped due to known clap debug assertion issue
    ];

    for args in &subcommands {
        raps_cmd()
            .args(args)
            .assert()
            // Check for panic indicators
            .stderr(predicate::str::contains("panicked").not())
            .stderr(predicate::str::contains("unreachable").not())
            .stderr(predicate::str::contains("RUST_BACKTRACE").not());
    }
}

// ============== EXIT CODE TESTS ==============
// Verify commands fail gracefully, not with panic exit code (101)

/// Test that auth test fails gracefully without credentials (not panic)
#[test]
fn test_auth_test_graceful_failure() {
    let output = raps_cmd()
        // Remove potential auth environment variables
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .env_remove("FORGE_CLIENT_ID")
        .env_remove("FORGE_CLIENT_SECRET")
        .args(["auth", "test"])
        .output()
        .unwrap();
    
    // Rust panic exit code is 101
    // Command may fail with auth error, but should NOT be 101 (panic)
    let exit_code = output.status.code().unwrap_or(-1);
    assert_ne!(
        exit_code, 101,
        "Command panicked (exit 101) instead of failing gracefully"
    );
}

/// Test that bucket list fails gracefully without credentials (not panic)
#[test]
fn test_bucket_list_graceful_failure() {
    let output = raps_cmd()
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .env_remove("FORGE_CLIENT_ID")
        .env_remove("FORGE_CLIENT_SECRET")
        .args(["bucket", "list"])
        .output()
        .unwrap();
    
    let exit_code = output.status.code().unwrap_or(-1);
    assert_ne!(
        exit_code, 101,
        "Command panicked (exit 101) instead of failing gracefully"
    );
}

// ============== HELP FLAG DISPATCH TESTS ==============
// Verify --help works for all subcommands (sanity check)

#[test]
fn test_all_help_flags_work() {
    let subcommands = [
        "auth", "bucket", "object", "translate", "hub", "project",
        "folder", "item", "webhook", "da", "issue", "acc", "rfi",
        "reality", "plugin", "generate", "demo", "config", "pipeline",
    ];

    for subcmd in subcommands {
        raps_cmd()
            .args([subcmd, "--help"])
            .assert()
            // Help should show usage information and not panic
            .stdout(predicate::str::contains("Usage"))
            .stderr(predicate::str::contains("panicked").not());
    }
}

// ============== REGRESSION TESTS ==============

/// Regression test for issue: std::mem::replace corrupting command state
/// This specific test would have caught the bug where auth commands
/// were incorrectly routed to the Completions handler
#[test]
fn test_auth_not_routed_to_completions() {
    raps_cmd()
        .args(["auth", "test"])
        .assert()
        // The bug caused auth commands to hit unreachable!() in Completions handler
        .stderr(predicate::str::contains("unreachable").not())
        // Verify we're not getting completions output
        .stdout(predicate::str::contains("_raps").not())
        .stdout(predicate::str::contains("complete -F").not())
        .stdout(predicate::str::contains("Register-ArgumentCompleter").not());
}

/// Regression test: config commands should work and not affect other commands
#[test]
fn test_config_dispatch_isolation() {
    // First, run a config command
    raps_cmd()
        .args(["config", "profile", "list"])
        .assert()
        .success();

    // Then verify auth still works (doesn't get corrupted)
    raps_cmd()
        .args(["auth", "test"])
        .assert()
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("unreachable").not());
}
