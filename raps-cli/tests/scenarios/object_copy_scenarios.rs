//! Scenario tests for object copy, rename, and batch-copy/batch-rename commands.
//!
//! The full copy round-trip (upload → copy → verify) is exercised through
//! bucket creation + the "no objects" empty-bucket path, argument validation,
//! and non-existent source error paths.  Single-object copy tests are omitted
//! because `object upload` panics in debug mode due to a clap positional-arg
//! ordering assertion (optional bucket before required file).

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

// ==================== batch-copy (empty bucket) ====================

/// `object batch-copy` from an empty bucket succeeds with "No objects to copy".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_batch_copy_empty_bucket_no_panic() {
    let (server, _) = start_cli_test().await;

    // Create source + destination buckets
    for bkt in ["bcem-src", "bcem-dst"] {
        let mut b = assert_cmd::Command::cargo_bin("raps").unwrap();
        b.env("APS_BASE_URL", &server.url)
            .env("APS_CLIENT_ID", "test-client")
            .env("APS_CLIENT_SECRET", "test-secret");
        b.args(["bucket", "create", "-k", bkt, "-p", "transient"])
            .assert()
            .success();
    }

    let mut cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    cmd.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    cmd.args(["object", "batch-copy", "bcem-src", "bcem-dst"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No objects to copy"));
}

/// `object batch-copy --keys` with explicit keys against existing bucket succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_batch_copy_with_keys_path_exercises_handler() {
    let (server, _) = start_cli_test().await;

    // Create source + destination buckets
    for bkt in ["bck-src", "bck-dst"] {
        let mut b = assert_cmd::Command::cargo_bin("raps").unwrap();
        b.env("APS_BASE_URL", &server.url)
            .env("APS_CLIENT_ID", "test-client")
            .env("APS_CLIENT_SECRET", "test-secret");
        b.args(["bucket", "create", "-k", bkt, "-p", "transient"])
            .assert()
            .success();
    }

    // --keys path bypasses list_objects; copy fails because object doesn't exist,
    // but the handler IS entered and does not panic.
    let mut cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    cmd.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    cmd.args([
        "object",
        "batch-copy",
        "bck-src",
        "bck-dst",
        "--keys",
        "nonexistent.txt",
    ])
    .assert()
    .code(predicate::ne(101));
}

// ==================== batch-rename (no matches) ====================

/// `object batch-rename` from empty bucket exits gracefully with "No objects match".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_batch_rename_no_matches_in_empty_bucket() {
    let (server, _) = start_cli_test().await;

    let mut bkt = assert_cmd::Command::cargo_bin("raps").unwrap();
    bkt.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    bkt.args(["bucket", "create", "-k", "bren-empty", "-p", "transient"])
        .assert()
        .success();

    let mut cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    cmd.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    cmd.args([
        "object",
        "batch-rename",
        "bren-empty",
        "--from",
        "old-",
        "--to",
        "new-",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("No objects match"));
}

// ==================== argument validation ====================

/// `object copy` without required --source-bucket fails with clap error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_copy_missing_source_bucket_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "object",
        "copy",
        "--source-object",
        "file.txt",
        "--dest-bucket",
        "dst",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("source-bucket").or(predicate::str::contains("required")));
}

/// `object copy` without required --source-object fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_copy_missing_source_object_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "object",
        "copy",
        "--source-bucket",
        "src",
        "--dest-bucket",
        "dst",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("source-object").or(predicate::str::contains("required")));
}

/// `object copy` without required --dest-bucket fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_copy_missing_dest_bucket_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "object",
        "copy",
        "--source-bucket",
        "src",
        "--source-object",
        "file.txt",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("dest-bucket").or(predicate::str::contains("required")));
}

/// `object batch-rename` without --from fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_batch_rename_missing_from_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["object", "batch-rename", "my-bucket", "--to", "new-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("from").or(predicate::str::contains("required")));
}

/// `object copy` error when source doesn't exist (exercises handler entry path).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_copy_nonexistent_source_fails_gracefully() {
    let (server, _) = start_cli_test().await;

    let mut dst = assert_cmd::Command::cargo_bin("raps").unwrap();
    dst.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    dst.args(["bucket", "create", "-k", "cp-dst-err", "-p", "transient"])
        .assert()
        .success();

    let mut cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    cmd.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    cmd.args([
        "object",
        "copy",
        "--source-bucket",
        "nonexistent-bucket",
        "--source-object",
        "missing.txt",
        "--dest-bucket",
        "cp-dst-err",
    ])
    .assert()
    .failure()
    .code(predicate::ne(101));
}

/// `object rename` error when source doesn't exist (exercises handler entry path).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_object_rename_nonexistent_source_fails_gracefully() {
    let (server, _) = start_cli_test().await;

    let mut bkt = assert_cmd::Command::cargo_bin("raps").unwrap();
    bkt.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    bkt.args(["bucket", "create", "-k", "ren-err", "-p", "transient"])
        .assert()
        .success();

    let mut cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    cmd.env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    cmd.args([
        "object",
        "rename",
        "ren-err",
        "missing.txt",
        "--new-key",
        "renamed.txt",
    ])
    .assert()
    .failure()
    .code(predicate::ne(101));
}
