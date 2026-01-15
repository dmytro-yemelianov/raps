use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_auth_status_output_flag() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("auth")
       .arg("status")
       .arg("--output")
       .arg("json")
       .assert()
       // Don't assert success as it might depend on env vars, but assert that it didn't crash
       // and output starts with '{' if successful, or error message if not.
       // However, if we're not logged in, it prints "Not logged in" in yellow if Table, 
       // or a JSON message if Json.
       .stdout(predicate::str::contains("{").or(predicate::str::contains("Not logged in"))); 
}

#[test]
fn test_da_engines_output_flag() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("da")
       .arg("engines")
       .arg("--output")
       .arg("json")
       .assert()
       // Might need auth, but ensuring the flag is accepted is the main goal here.
       .stderr(predicate::str::contains("invalid value").not());
}

#[test]
fn test_config_list_output_flag() {
    let mut cmd = Command::cargo_bin("raps").unwrap();
    cmd.arg("config")
       .arg("profile")
       .arg("list")
       .arg("--output")
       .arg("yaml")
       .assert()
       // Should definitely work without net auth
       .success();
       // .stdout(predicate::str::contains("default:")); // YAML format
}
