use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_command_runs() {
    Command::cargo_bin("raps")
        .expect("binary should build")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("raps"));
}

#[test]
fn config_profile_list_succeeds_without_credentials() {
    Command::cargo_bin("raps")
        .expect("binary should build")
        .args(["config", "profile", "list", "--output", "json"])
        .assert()
        .success();
}

#[test]
fn bucket_create_missing_args_returns_usage_error() {
    Command::cargo_bin("raps")
        .expect("binary should build")
        .args(["bucket", "create"])
        .assert()
        .failure()
        .code(2);
}
