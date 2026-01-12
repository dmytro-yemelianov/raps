//! Integration tests for config commands
//!
//! Tests CLI argument parsing, help output, and error handling for config commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_config_help() {
    raps()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration"))
        .stdout(predicate::str::contains("profile"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("set"));
}

#[test]
fn test_config_profile_help() {
    raps()
        .args(["config", "profile", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("profile"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("use"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn test_config_profile_create_help() {
    raps()
        .args(["config", "profile", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create a new profile"))
        .stdout(predicate::str::contains("NAME"));
}

#[test]
fn test_config_profile_use_help() {
    raps()
        .args(["config", "profile", "use", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("active profile"))
        .stdout(predicate::str::contains("NAME"));
}

#[test]
fn test_config_profile_delete_help() {
    raps()
        .args(["config", "profile", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete"))
        .stdout(predicate::str::contains("NAME"));
}

#[test]
fn test_config_profile_export_help() {
    raps()
        .args(["config", "profile", "export", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Export"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--include-secrets"));
}

#[test]
fn test_config_profile_import_help() {
    raps()
        .args(["config", "profile", "import", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Import"))
        .stdout(predicate::str::contains("FILE"))
        .stdout(predicate::str::contains("--overwrite"));
}

#[test]
fn test_config_get_help() {
    raps()
        .args(["config", "get", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Get a configuration"))
        .stdout(predicate::str::contains("KEY"));
}

#[test]
fn test_config_set_help() {
    raps()
        .args(["config", "set", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set a configuration"))
        .stdout(predicate::str::contains("KEY"))
        .stdout(predicate::str::contains("VALUE"));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_config_profile_create_requires_name() {
    raps()
        .args(["config", "profile", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NAME").or(predicate::str::contains("required")));
}

#[test]
fn test_config_profile_use_requires_name() {
    raps()
        .args(["config", "profile", "use"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NAME").or(predicate::str::contains("required")));
}

#[test]
fn test_config_profile_delete_requires_name() {
    raps()
        .args(["config", "profile", "delete"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NAME").or(predicate::str::contains("required")));
}

#[test]
fn test_config_get_requires_key() {
    raps()
        .args(["config", "get"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("KEY").or(predicate::str::contains("required")));
}

#[test]
fn test_config_set_requires_key_and_value() {
    raps()
        .args(["config", "set"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("KEY").or(predicate::str::contains("required")));
}

#[test]
fn test_config_profile_import_requires_file() {
    raps()
        .args(["config", "profile", "import"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FILE").or(predicate::str::contains("required")));
}

// ==================== Profile Operations ====================

#[test]
fn test_config_profile_list_works() {
    // List should work even without any profiles
    raps()
        .args(["config", "profile", "list"])
        .assert()
        .success();
}

#[test]
fn test_config_profile_current_works() {
    // Current should work even without active profile
    raps()
        .args(["config", "profile", "current"])
        .assert()
        .success();
}
