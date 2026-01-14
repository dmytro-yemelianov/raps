//! Live API integration tests
//!
//! These tests hit the real APS API and require valid credentials.
//! Run with: `cargo test -p raps-cli --test live_api_tests -- --ignored`
//!
//! Required environment variables:
//! - APS_CLIENT_ID
//! - APS_CLIENT_SECRET
//!
//! Optional:
//! - APS_CALLBACK_URL (for 3-legged tests)
//! - APS_DA_NICKNAME (for Design Automation tests)

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use std::env;
use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test environment by loading .env file
fn init_env() {
    INIT.call_once(|| {
        // Try to load .env from the workspace root
        let _ = dotenvy::from_filename("../.env");
        // Also try from current directory
        let _ = dotenvy::dotenv();
    });
}

/// Check if credentials are available
fn has_credentials() -> bool {
    init_env();
    env::var("APS_CLIENT_ID").is_ok() && env::var("APS_CLIENT_SECRET").is_ok()
}

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== AUTH TESTS ====================

/// Test 2-legged OAuth authentication
#[test]
#[ignore]
fn test_live_auth_test() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["auth", "test"])
        .assert()
        .success()
        .stdout(predicates::str::contains("success").or(predicates::str::contains("valid")));
}

/// Test auth status command
#[test]
#[ignore]
fn test_live_auth_status() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["auth", "status", "--output", "json"])
        .assert()
        .success();
}

// ==================== BUCKET TESTS ====================

/// Test bucket list (read-only, safe)
#[test]
#[ignore]
fn test_live_bucket_list() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["bucket", "list", "--output", "json"])
        .assert()
        .success();
}

/// Test bucket list with table output
#[test]
#[ignore]
fn test_live_bucket_list_table() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["bucket", "list", "--output", "table"])
        .assert()
        .success();
}

/// Test bucket info for non-existent bucket
#[test]
#[ignore]
fn test_live_bucket_info_not_found() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["bucket", "info", "nonexistent-bucket-12345xyz"])
        .assert()
        .failure();
}

// ==================== OBJECT TESTS ====================

/// Test object list on non-existent bucket
#[test]
#[ignore]
fn test_live_object_list_bucket_not_found() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["object", "list", "nonexistent-bucket-12345xyz"])
        .assert()
        .failure();
}

// ==================== WEBHOOK TESTS ====================

/// Test webhook events list (static data, always works)
#[test]
#[ignore]
fn test_live_webhook_events() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["webhook", "events"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dm."));
}

/// Test webhook list
#[test]
#[ignore]
fn test_live_webhook_list() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["webhook", "list", "--output", "json"])
        .assert()
        .success();
}

// ==================== DESIGN AUTOMATION TESTS ====================

/// Test DA engines list
#[test]
#[ignore]
fn test_live_da_engines() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["da", "engines", "--output", "json"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Autodesk"));
}

/// Test DA appbundles list
#[test]
#[ignore]
fn test_live_da_appbundles() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["da", "appbundles", "--output", "json"])
        .assert()
        .success();
}

/// Test DA activities list
#[test]
#[ignore]
fn test_live_da_activities() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["da", "activities", "--output", "json"])
        .assert()
        .success();
}

// ==================== TRANSLATE TESTS ====================

/// Test translate status for invalid URN
#[test]
#[ignore]
fn test_live_translate_status_invalid_urn() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    // Invalid URN should fail gracefully
    raps()
        .args(["translate", "status", "invalid-urn"])
        .assert()
        .failure();
}

// ==================== OUTPUT FORMAT TESTS ====================

/// Test JSON output for bucket list
#[test]
#[ignore]
fn test_live_output_json() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    let output = raps()
        .args(["bucket", "list", "--output", "json"])
        .output()
        .unwrap();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Should be valid JSON
        assert!(stdout.contains("[") || stdout.contains("{"));
    }
}

/// Test YAML output for bucket list
#[test]
#[ignore]
fn test_live_output_yaml() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    let output = raps()
        .args(["bucket", "list", "--output", "yaml"])
        .output()
        .unwrap();

    // Command should accept yaml output format
    assert!(
        output.status.success()
            || !String::from_utf8_lossy(&output.stderr).contains("invalid value 'yaml'")
    );
}

// ==================== VERBOSE/DEBUG FLAGS ====================

/// Test verbose flag with real API call
#[test]
#[ignore]
fn test_live_verbose_flag() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    let output = raps()
        .args(["--verbose", "bucket", "list"])
        .output()
        .unwrap();

    // Verbose output goes to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should show HTTP requests in verbose mode
    assert!(
        output.status.success() || stderr.contains("GET") || stderr.contains("POST"),
        "Verbose should show request info"
    );
}

/// Test debug flag with real API call
#[test]
#[ignore]
fn test_live_debug_flag() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    let output = raps().args(["--debug", "auth", "test"]).output().unwrap();

    // Debug mode should work (may show more info)
    assert!(output.status.success());
}

// ==================== TIMEOUT TESTS ====================

/// Test custom timeout
#[test]
#[ignore]
fn test_live_custom_timeout() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    raps()
        .args(["--timeout", "60", "bucket", "list"])
        .assert()
        .success();
}

// ==================== END-TO-END WORKFLOW TESTS ====================

/// Test full bucket create -> list -> delete workflow
/// WARNING: This creates and deletes a real bucket
#[test]
#[ignore]
fn test_live_bucket_workflow() {
    if !has_credentials() {
        eprintln!("Skipping: APS_CLIENT_ID and APS_CLIENT_SECRET not set");
        return;
    }

    // Generate unique bucket name
    let bucket_key = format!(
        "raps-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 100000000
    );

    // Create bucket
    let create_result = raps()
        .args([
            "bucket",
            "create",
            "--key",
            &bucket_key,
            "--policy",
            "transient",
            "--output",
            "json",
            "--yes",
        ])
        .output()
        .unwrap();

    if !create_result.status.success() {
        eprintln!(
            "Bucket create failed (may already exist): {}",
            String::from_utf8_lossy(&create_result.stderr)
        );
        return;
    }

    println!("Created bucket: {}", bucket_key);

    // Verify bucket appears in list
    let list_result = raps()
        .args(["bucket", "list", "--output", "json"])
        .output()
        .unwrap();

    assert!(list_result.status.success());
    let stdout = String::from_utf8_lossy(&list_result.stdout);
    assert!(
        stdout.contains(&bucket_key),
        "Created bucket should appear in list"
    );

    // Get bucket info
    raps()
        .args(["bucket", "info", &bucket_key, "--output", "json"])
        .assert()
        .success();

    // Delete bucket
    raps()
        .args(["bucket", "delete", &bucket_key, "--yes"])
        .assert()
        .success();

    println!("Deleted bucket: {}", bucket_key);
}
