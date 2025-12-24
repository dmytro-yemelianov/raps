//! Integration tests for APS CLI
//!
//! These tests verify end-to-end functionality and require proper configuration.
//! Most tests are marked with #[ignore] and should be run explicitly with:
//! `cargo test -- --ignored`
//!
//! Note: Integration tests that require actual API access should be run manually
//! with proper credentials configured.

use std::process::Command;

/// Test that the CLI binary can be executed and shows help
#[test]
#[ignore] // Requires binary to be built
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "CLI should show help successfully");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("raps"), "Help should contain 'raps'");
    assert!(
        stdout.contains("Command-line interface"),
        "Help should contain description"
    );
}

/// Test that the CLI shows version information
#[test]
#[ignore] // Requires binary to be built
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "CLI should show version successfully"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("raps"),
        "Version output should contain 'raps'"
    );
    assert!(
        stdout.contains("0.3.0"),
        "Version output should contain version number"
    );
}

/// Test that invalid commands show appropriate error messages
#[test]
#[ignore] // Requires binary to be built
fn test_cli_invalid_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "invalid-command"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "Invalid command should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error") || stderr.contains("Invalid"),
        "Error output should indicate failure"
    );
}
