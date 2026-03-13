// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for CLI input validation at command boundaries.
//!
//! These tests verify that user-supplied IDs with injection characters
//! are rejected before any API calls are made.

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;

#[test]
fn test_input_validation_bucket_rejects_query_injection() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.args(["bucket", "info", "key?injected=true"]);
    cmd.assert().failure().stderr(
        predicates::str::contains("Invalid bucket key")
            .or(predicates::str::contains("query-parameter")),
    );
}

#[test]
fn test_input_validation_bucket_rejects_fragment() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.args(["bucket", "info", "key#fragment"]);
    cmd.assert().failure().stderr(
        predicates::str::contains("Invalid bucket key")
            .or(predicates::str::contains("query-parameter")
                .or(predicates::str::contains("Invalid"))),
    );
}

#[test]
fn test_input_validation_hub_info_rejects_injection() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.args(["hub", "info", "hub?injected=true"]);
    cmd.assert().failure().stderr(
        predicates::str::contains("Invalid").or(predicates::str::contains("query-parameter")),
    );
}

#[test]
fn test_input_validation_project_list_rejects_injection() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.args(["project", "list", "hub&injected=true"]);
    cmd.assert().failure().stderr(
        predicates::str::contains("Invalid").or(predicates::str::contains("query-parameter")),
    );
}
