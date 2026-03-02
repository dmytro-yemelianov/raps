//! Integration tests for swarm subcommands.
//!
//! Tests help output, subcommand presence, and graceful failure modes
//! for the distributed worker CLI. Worker-specific tests require the
//! `redis` feature flag.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Swarm Top-Level ====================

#[test]
fn test_swarm_subcommands_exist() {
    raps()
        .args(["swarm", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("metrics"))
        .stdout(predicate::str::contains("queue"))
        .stdout(predicate::str::contains("resume"))
        .stdout(predicate::str::contains("audit"))
        .stdout(predicate::str::contains("reset"));
}

#[test]
fn test_swarm_status_help() {
    raps()
        .args(["swarm", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("circuit breaker").or(predicate::str::contains("rate")));
}

// ==================== Worker (redis feature) ====================

#[cfg(feature = "redis")]
#[test]
fn test_swarm_worker_help() {
    raps()
        .args(["swarm", "worker", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("start").or(predicate::str::contains("Start")));
}

#[cfg(feature = "redis")]
#[test]
fn test_swarm_worker_start_help() {
    raps()
        .args(["swarm", "worker", "start", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--redis-url"))
        .stdout(predicate::str::contains("--concurrency"))
        .stdout(predicate::str::contains("--heartbeat-secs"))
        .stdout(predicate::str::contains("--metrics-port"));
}

#[cfg(feature = "redis")]
#[test]
fn test_swarm_worker_start_missing_redis() {
    // Worker start should fail gracefully when Redis is unreachable.
    // Use a bogus URL to ensure no accidental connection to a real instance.
    raps()
        .args([
            "swarm",
            "worker",
            "start",
            "--redis-url",
            "redis://127.0.0.1:1",
        ])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .failure();
}
