use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_starts_successfully() {
    let mut command = Command::cargo_bin("quickbase").expect("binary exists");

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Query and operate against the Quickbase REST API",
        ));
}

#[test]
fn help_lists_required_top_level_commands() {
    let mut command = Command::cargo_bin("quickbase").expect("binary exists");

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("cmd"))
        .stdout(predicate::str::contains("server"))
        .stdout(predicate::str::contains("util"));
}

#[test]
fn subcommand_help_is_available() {
    for subcommand in ["cmd", "server", "util"] {
        let mut command = Command::cargo_bin("quickbase").expect("binary exists");

        command
            .args([subcommand, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--json"))
            .stdout(predicate::str::contains("--markdown"));
    }
}
