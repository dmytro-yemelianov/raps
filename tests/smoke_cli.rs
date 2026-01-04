use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;

#[test]
fn help_command_runs() {
    cargo_bin_cmd!("raps")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("raps"));
}

#[test]
fn config_profile_list_succeeds_without_credentials() {
    cargo_bin_cmd!("raps")
        .args(["config", "profile", "list", "--output", "json"])
        .assert()
        .success();
}

#[test]
fn bucket_info_missing_args_returns_usage_error() {
    cargo_bin_cmd!("raps")
        .args(["bucket", "info"])
        .assert()
        .failure()
        .code(2);
}
