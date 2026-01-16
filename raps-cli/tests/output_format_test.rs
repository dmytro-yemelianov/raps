use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_output_flag_yaml_help() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--output <FORMAT>"));
}

// NOTE: We cannot fully test actual YAML output until a command is wired up to use it (T011).
// This test is a placeholder that validates the flag exists and is accepted by clap.
#[test]
fn test_output_flag_accepts_yaml() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    // Use a command that shouldn't require auth for help/parsing check, or fail gracefully
    cmd.arg("bucket")
        .arg("list")
        .arg("--output")
        .arg("yaml")
        .assert()
        // It might fail due to missing auth/config, but it shouldn't fail due to "invalid value for '--output'"
        .stderr(predicate::str::contains("invalid value").not());
}
