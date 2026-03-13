use assert_cmd::Command;
use predicates::str;

#[test]
fn test_docs_mcp_exits_zero() {
    Command::cargo_bin("raps")
        .unwrap()
        .args(["docs", "mcp"])
        .assert()
        .success();
}

#[test]
fn test_docs_mcp_output_contains_tool_table() {
    let output = Command::cargo_bin("raps")
        .unwrap()
        .args(["docs", "mcp"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("auth_test"), "missing auth_test tool");
    assert!(stdout.contains("bucket_list"), "missing bucket_list tool");
    assert!(
        stdout.contains("2-leg") || stdout.contains("2-legged"),
        "missing auth type info"
    );
    assert!(stdout.contains("## Tools"), "missing Tools section header");
    assert!(
        stdout.contains("## Agent Invariants"),
        "missing Agent Invariants section"
    );
}

#[test]
fn test_docs_mcp_check_flag_passes_when_agents_md_matches() {
    // Generate AGENTS.md first, then verify --check passes
    Command::cargo_bin("raps")
        .unwrap()
        .args(["docs", "mcp", "--write"])
        .current_dir("/root/github/raps/raps/.worktrees/agent-first-cli")
        .assert()
        .success();

    Command::cargo_bin("raps")
        .unwrap()
        .args(["docs", "mcp", "--check"])
        .current_dir("/root/github/raps/raps/.worktrees/agent-first-cli")
        .assert()
        .success();
}
