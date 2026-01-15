use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_webhook_create_non_interactive_missing_args() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("webhook")
       .arg("create")
       .arg("--non-interactive")
       // Missing --url and --event
       .assert()
       .failure()
       // Should fail because URL is required and has no default
       .stderr(predicate::str::contains("required in non-interactive mode"));
}

#[test]
fn test_bucket_create_non_interactive_success() {
    // This requires a mock server or actual API, which we might not have in this environment.
    // But we can test that it doesn't fail with "required in non-interactive mode".
    // It might fail with auth error or API error, which is fine.
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("bucket")
       .arg("create")
       .arg("--non-interactive")
       .arg("--key").arg("test-bucket-12345")
       .assert()
       // It won't prompt. If auth fails (exit 3), that means it passed argument validation.
       .stderr(predicate::str::contains("required in non-interactive mode").not());
}