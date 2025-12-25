//! Integration tests for RAPS CLI
//!
//! These tests verify end-to-end functionality and CLI behavior.
//! Most tests are marked with #[ignore] and should be run explicitly with:
//! `cargo test -- --ignored`
//!
//! Note: Integration tests that require actual API access should be run manually
//! with proper credentials configured.

use std::process::Command;

// ============== BASIC CLI TESTS ==============

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
        stdout.contains("1.0.0"),
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

// ============== EXIT CODE TESTS ==============

/// Test that invalid arguments return exit code 2
#[test]
#[ignore]
fn test_exit_code_invalid_args() {
    let output = Command::new("cargo")
        .args(["run", "--", "bucket", "create"])
        .output()
        .expect("Failed to execute command");

    // Missing required arguments should return exit code 2
    assert_eq!(
        output.status.code(),
        Some(2),
        "Invalid arguments should return exit code 2"
    );
}

// ============== OUTPUT FORMAT TESTS ==============

/// Test JSON output format flag
#[test]
#[ignore]
fn test_output_format_json() {
    let output = Command::new("cargo")
        .args(["run", "--", "--output", "json", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "JSON output flag should be accepted"
    );
}

/// Test YAML output format flag
#[test]
#[ignore]
fn test_output_format_yaml() {
    let output = Command::new("cargo")
        .args(["run", "--", "--output", "yaml", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "YAML output flag should be accepted"
    );
}

/// Test table output format flag
#[test]
#[ignore]
fn test_output_format_table() {
    let output = Command::new("cargo")
        .args(["run", "--", "--output", "table", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Table output flag should be accepted"
    );
}

// ============== GLOBAL FLAGS TESTS ==============

/// Test --no-color flag
#[test]
#[ignore]
fn test_no_color_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--no-color", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "--no-color flag should be accepted"
    );
}

/// Test --quiet flag
#[test]
#[ignore]
fn test_quiet_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--quiet", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "--quiet flag should be accepted");
}

/// Test --verbose flag
#[test]
#[ignore]
fn test_verbose_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--verbose", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "--verbose flag should be accepted");
}

/// Test --debug flag
#[test]
#[ignore]
fn test_debug_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--debug", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "--debug flag should be accepted");
}

/// Test --non-interactive flag
#[test]
#[ignore]
fn test_non_interactive_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--non-interactive", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "--non-interactive flag should be accepted"
    );
}

/// Test --timeout flag
#[test]
#[ignore]
fn test_timeout_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--timeout", "60", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "--timeout flag should be accepted");
}

/// Test --concurrency flag
#[test]
#[ignore]
fn test_concurrency_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--concurrency", "10", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "--concurrency flag should be accepted"
    );
}

// ============== SUBCOMMAND HELP TESTS ==============

/// Test auth subcommand help
#[test]
#[ignore]
fn test_auth_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "auth", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "auth --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("login"), "auth help should mention login");
    assert!(stdout.contains("logout"), "auth help should mention logout");
}

/// Test bucket subcommand help
#[test]
#[ignore]
fn test_bucket_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "bucket", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "bucket --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "bucket help should mention list");
    assert!(
        stdout.contains("create"),
        "bucket help should mention create"
    );
}

/// Test object subcommand help
#[test]
#[ignore]
fn test_object_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "object", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "object --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("upload"),
        "object help should mention upload"
    );
    assert!(
        stdout.contains("download"),
        "object help should mention download"
    );
}

/// Test translate subcommand help
#[test]
#[ignore]
fn test_translate_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "translate", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "translate --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("start"),
        "translate help should mention start"
    );
    assert!(
        stdout.contains("status"),
        "translate help should mention status"
    );
}

/// Test rfi subcommand help
#[test]
#[ignore]
fn test_rfi_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "rfi", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "rfi --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "rfi help should mention list");
    assert!(stdout.contains("create"), "rfi help should mention create");
}

/// Test acc subcommand help
#[test]
#[ignore]
fn test_acc_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "acc", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "acc --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("asset"), "acc help should mention asset");
    assert!(
        stdout.contains("submittal"),
        "acc help should mention submittal"
    );
    assert!(
        stdout.contains("checklist"),
        "acc help should mention checklist"
    );
}

/// Test plugin subcommand help
#[test]
#[ignore]
fn test_plugin_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "plugin", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "plugin --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "plugin help should mention list");
    assert!(
        stdout.contains("enable"),
        "plugin help should mention enable"
    );
    assert!(
        stdout.contains("disable"),
        "plugin help should mention disable"
    );
}

/// Test config subcommand help
#[test]
#[ignore]
fn test_config_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "config", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "config --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("profile"),
        "config help should mention profile"
    );
}

/// Test pipeline subcommand help
#[test]
#[ignore]
fn test_pipeline_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "pipeline", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "pipeline --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("run"), "pipeline help should mention run");
    assert!(
        stdout.contains("validate"),
        "pipeline help should mention validate"
    );
}

// ============== SHELL COMPLETION TESTS ==============

/// Test bash completions generation
#[test]
#[ignore]
fn test_completions_bash() {
    let output = Command::new("cargo")
        .args(["run", "--", "completions", "bash"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "bash completions should generate");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("_raps") || stdout.contains("complete"),
        "Output should contain bash completion syntax"
    );
}

/// Test powershell completions generation
#[test]
#[ignore]
fn test_completions_powershell() {
    let output = Command::new("cargo")
        .args(["run", "--", "completions", "powershell"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "powershell completions should generate"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Register-ArgumentCompleter") || stdout.contains("raps"),
        "Output should contain PowerShell completion syntax"
    );
}

/// Test zsh completions generation
#[test]
#[ignore]
fn test_completions_zsh() {
    let output = Command::new("cargo")
        .args(["run", "--", "completions", "zsh"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success(), "zsh completions should generate");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("#compdef") || stdout.contains("_raps"),
        "Output should contain zsh completion syntax"
    );
}

// ============== CONFIG COMMAND TESTS ==============

/// Test config profile list (doesn't require auth)
#[test]
#[ignore]
fn test_config_profile_list() {
    let output = Command::new("cargo")
        .args(["run", "--", "config", "profile", "list", "--output", "json"])
        .output()
        .expect("Failed to execute command");

    // This should work without authentication
    assert!(
        output.status.success(),
        "config profile list should succeed without auth"
    );
}

/// Test config profile current (doesn't require auth)
#[test]
#[ignore]
fn test_config_profile_current() {
    let output = Command::new("cargo")
        .args(["run", "--", "config", "profile", "current"])
        .output()
        .expect("Failed to execute command");

    // This should work without authentication
    assert!(
        output.status.success(),
        "config profile current should succeed without auth"
    );
}

// ============== PLUGIN COMMAND TESTS ==============

/// Test plugin list (doesn't require auth)
#[test]
#[ignore]
fn test_plugin_list() {
    let output = Command::new("cargo")
        .args(["run", "--", "plugin", "list", "--output", "json"])
        .output()
        .expect("Failed to execute command");

    // Plugin list should work without authentication
    assert!(
        output.status.success(),
        "plugin list should succeed without auth"
    );
}

/// Test plugin alias list (doesn't require auth)
#[test]
#[ignore]
fn test_plugin_alias_list() {
    let output = Command::new("cargo")
        .args(["run", "--", "plugin", "alias", "list", "--output", "json"])
        .output()
        .expect("Failed to execute command");

    // Alias list should work without authentication
    assert!(
        output.status.success(),
        "plugin alias list should succeed without auth"
    );
}
