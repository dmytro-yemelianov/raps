use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_no_color_flag() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    // Use a command that typically has color output, e.g., version or help
    cmd.arg("--version").arg("--no-color").assert().success();

    // Hard to test absence of ANSI codes without regex, but we can verify flag is accepted.
}

#[test]
fn test_quiet_flag() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    // `raps config profile list` prints "Active profile: ..." or "Profiles:"
    // With --quiet, it might still print result but suppress "Fetching...".
    // Since config commands are local, they don't have "Fetching..." logs usually.
    // But we can verify flag acceptance.
    cmd.arg("config")
        .arg("profile")
        .arg("list")
        .arg("--quiet")
        .assert()
        .success();
}
