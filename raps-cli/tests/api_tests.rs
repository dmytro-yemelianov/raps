// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for the custom API command

use assert_cmd::Command;
use predicates::prelude::*;

/// Helper to create a raps command
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_api_help() {
    raps()
        .arg("api")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute custom API calls"));
}

#[test]
fn test_api_get_help() {
    raps()
        .args(["api", "get", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute HTTP GET request"))
        .stdout(predicate::str::contains("--query"))
        .stdout(predicate::str::contains("--header"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_api_post_help() {
    raps()
        .args(["api", "post", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute HTTP POST request"))
        .stdout(predicate::str::contains("--data"))
        .stdout(predicate::str::contains("--data-file"));
}

#[test]
fn test_api_put_help() {
    raps()
        .args(["api", "put", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute HTTP PUT request"));
}

#[test]
fn test_api_patch_help() {
    raps()
        .args(["api", "patch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute HTTP PATCH request"));
}

#[test]
fn test_api_delete_help() {
    raps()
        .args(["api", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute HTTP DELETE request"));
}

#[test]
fn test_api_get_missing_endpoint() {
    raps()
        .args(["api", "get"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_api_post_data_file_conflict() {
    // --data and --data-file should conflict
    raps()
        .args(["api", "post", "/test", "--data", "{}", "--data-file", "test.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_api_get_invalid_query_format() {
    // Query params should be KEY=VALUE format
    raps()
        .args(["api", "get", "/test", "--query", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid format"));
}

#[test]
fn test_api_get_invalid_header_format() {
    // Headers should be KEY:VALUE format
    raps()
        .args(["api", "get", "/test", "--header", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid header format"));
}
