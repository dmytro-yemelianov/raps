use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_exit_code_invalid_args() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("--invalid-flag")
       .assert()
       .failure()
       .code(2);
}

#[test]
fn test_exit_code_success() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("--version")
       .assert()
       .success()
       .code(0);
}

// We can't easily test 3, 4, 5 without full mocking or live API which assert_cmd doesn't do easily.
// But we can verify the mechanism is in place if T004 is implemented.
