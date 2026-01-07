use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore = "Requires binary to be built and doesn't work well with cargo nextest/llvm-cov"]
fn test_enhanced_shell_welcome_message() {
    let mut cmd = Command::new("cargo");
    let mut child = cmd
        .args(&["run", "--bin", "raps", "--", "shell"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"exit\n").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains("Welcome to the RAPS interactive shell!"));
    assert!(stdout.contains("Use TAB for command completion"));
    assert!(stdout.contains("hints show required parameters"));
}

#[test]
#[ignore = "Requires binary to be built and doesn't work well with cargo nextest/llvm-cov"]
fn test_shell_help_command() {
    let mut cmd = Command::new("cargo");
    let mut child = cmd
        .args(&["run", "--bin", "raps", "--", "shell"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"help\nexit\n").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains("Available commands:"));
    assert!(stdout.contains("auth"));
    assert!(stdout.contains("bucket"));
    assert!(stdout.contains("object"));
    assert!(stdout.contains("Press TAB for command completion"));
}