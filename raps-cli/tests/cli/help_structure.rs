//! Snapshot tests for the CLI command tree.
//! Catch accidental renames, removals, or flag changes.

use assert_cmd::Command;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_admin_help_snapshot() {
    let output = raps().args(["admin", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_help", stdout);
}

#[test]
fn test_admin_user_help_snapshot() {
    let output = raps().args(["admin", "user", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_user_help", stdout);
}

#[test]
fn test_admin_project_help_snapshot() {
    let output = raps()
        .args(["admin", "project", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_project_help", stdout);
}

#[test]
fn test_admin_user_add_to_all_projects_help_snapshot() {
    let output = raps()
        .args(["admin", "user", "add-to-all-projects", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    insta::assert_snapshot!("admin_user_add_to_all_projects_help", stdout);
}
