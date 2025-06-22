use std::{
    fs,
    path::{Path, PathBuf},
};

use assert_cmd::Command;
use predicates::prelude::*;
use quickbase_cli::config::{ConfigMode, parse_config};
use tempfile::TempDir;

struct TestRepo {
    _temp: TempDir,
    root: PathBuf,
    nested: PathBuf,
}

impl TestRepo {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp repo");
        let root = temp.path().to_path_buf();
        fs::create_dir(root.join(".git")).expect("git marker");
        let nested = root.join("nested").join("workdir");
        fs::create_dir_all(&nested).expect("nested workdir");

        Self {
            _temp: temp,
            root,
            nested,
        }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn config_path(&self) -> PathBuf {
        self.root.join(".quickbase").join("quickbase.jsonc")
    }

    fn gitignore_path(&self) -> PathBuf {
        self.root.join(".quickbase").join(".gitignore")
    }

    fn write_config(&self, contents: &str) {
        let config_dir = self.root.join(".quickbase");
        fs::create_dir_all(&config_dir).expect("config dir");
        fs::write(config_dir.join("quickbase.jsonc"), contents).expect("config file");
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("quickbase").expect("binary exists");
        command.current_dir(&self.nested);
        command
    }
}

#[test]
fn valid_jsonc_with_comments_passes() {
    let config = parse_config(include_str!("fixtures/config/valid.jsonc"))
        .expect("valid JSONC config parses");

    assert_eq!(config.quickbase_realm, "example.quickbase.com");
    assert_eq!(config.app_id, "app_fixture");
    assert_eq!(config.quickbase_user_token, "test-token");
    assert_eq!(config.mode, ConfigMode::Dryrun);
}

#[test]
fn missing_required_fields_fail() {
    for (field, text) in [
        (
            "quickbaseAppId",
            r#"{"quickbaseRealm":"example.quickbase.com","quickbaseUserToken":"token","mode":"dryrun"}"#,
        ),
        (
            "quickbaseRealm",
            r#"{"quickbaseAppId":"app_fixture","quickbaseUserToken":"token","mode":"dryrun"}"#,
        ),
        (
            "quickbaseUserToken",
            r#"{"quickbaseRealm":"example.quickbase.com","quickbaseAppId":"app_fixture","mode":"dryrun"}"#,
        ),
        (
            "mode",
            r#"{"quickbaseRealm":"example.quickbase.com","quickbaseAppId":"app_fixture","quickbaseUserToken":"token"}"#,
        ),
    ] {
        let error = parse_config(text).expect_err("missing field fails");

        assert!(
            error.to_string().contains(field),
            "expected error to mention {field}, got {error}"
        );
    }
}

#[test]
fn invalid_mode_fails() {
    let error = parse_config(include_str!("fixtures/config/invalid-mode.jsonc"))
        .expect_err("invalid mode fails");

    assert!(error.to_string().contains("mode"));
    assert!(error.to_string().contains("dryrun"));
}

#[test]
fn full_url_realm_fails() {
    let error = parse_config(
        r#"{
          "quickbaseRealm": "https://example.quickbase.com",
          "quickbaseAppId": "app_fixture",
          "quickbaseUserToken": "token",
          "mode": "dryrun"
        }"#,
    )
    .expect_err("full URL realm fails");

    assert!(error.to_string().contains("not a full URL"));
}

#[test]
fn make_config_creates_default_file_and_gitignore_under_repo_root() {
    let repo = TestRepo::new();

    repo.command()
        .args(["util", "make-config", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            repo.config_path().display().to_string(),
        ))
        .stdout(predicate::str::contains(
            repo.gitignore_path().display().to_string(),
        ))
        .stdout(predicate::str::contains("\"created\": true"))
        .stdout(predicate::str::contains("\"alreadyExisted\": false"))
        .stdout(predicate::str::contains("\"gitignoreCreated\": true"))
        .stdout(predicate::str::contains(
            "\"gitignoreAlreadyExisted\": false",
        ));

    let contents = fs::read_to_string(repo.config_path()).expect("generated config exists");
    assert!(contents.contains(r#""quickbaseAppId": "replace-with-your-app-id""#));
    assert!(contents.contains(r#""mode": "dryrun""#));
    assert!(contents.contains("//"));
    assert_eq!(
        fs::read_to_string(repo.gitignore_path()).expect("generated gitignore exists"),
        "*\n!.gitignore\n"
    );
}

#[test]
fn make_config_does_not_overwrite_existing_file() {
    let repo = TestRepo::new();
    let config_dir = repo.root().join(".quickbase");
    let config_path = repo.config_path();
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(&config_path, "sentinel").expect("existing config");

    repo.command()
        .args(["util", "make-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"created\": false"))
        .stdout(predicate::str::contains("\"alreadyExisted\": true"))
        .stdout(predicate::str::contains("\"gitignoreCreated\": true"));

    assert_eq!(
        fs::read_to_string(config_path).expect("existing config remains"),
        "sentinel"
    );
}

#[test]
fn make_config_does_not_overwrite_existing_gitignore() {
    let repo = TestRepo::new();
    let config_dir = repo.root().join(".quickbase");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::write(repo.gitignore_path(), "sentinel\n").expect("existing gitignore");

    repo.command()
        .args(["util", "make-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"gitignoreCreated\": false"))
        .stdout(predicate::str::contains(
            "\"gitignoreAlreadyExisted\": true",
        ));

    assert_eq!(
        fs::read_to_string(repo.gitignore_path()).expect("existing gitignore remains"),
        "sentinel\n"
    );
}

#[test]
fn validate_config_succeeds_without_printing_token() {
    let repo = TestRepo::new();
    repo.write_config(
        r#"{
          // comments are accepted
          "quickbaseRealm": "example.quickbase.com",
          "quickbaseAppId": "app_validate",
          "quickbaseUserToken": "secret-token",
          "mode": "live"
        }"#,
    );

    repo.command()
        .args(["util", "validate-config", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            repo.config_path().display().to_string(),
        ))
        .stdout(predicate::str::contains("\"valid\": true"))
        .stdout(predicate::str::contains(
            "\"quickbaseRealm\": \"example.quickbase.com\"",
        ))
        .stdout(predicate::str::contains(
            "\"quickbaseAppId\": \"app_validate\"",
        ))
        .stdout(predicate::str::contains("\"mode\": \"live\""))
        .stdout(predicate::str::contains("secret-token").not());
}

#[test]
fn validate_config_fails_for_invalid_config() {
    let repo = TestRepo::new();
    repo.write_config(
        r#"{
          "quickbaseRealm": "example.quickbase.com",
          "quickbaseAppId": "app_fixture",
          "quickbaseUserToken": "token",
          "mode": "invalid"
        }"#,
    );

    repo.command()
        .args(["util", "validate-config"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("mode"))
        .stderr(predicate::str::contains("dryrun"));
}

#[test]
fn config_commands_fail_outside_git_repo() {
    let dir = TempDir::new().expect("temp dir");
    let mut command = Command::cargo_bin("quickbase").expect("binary exists");

    command
        .current_dir(dir.path())
        .args(["util", "validate-config"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "must be run inside a Git work tree",
        ))
        .stderr(predicate::str::contains(
            "<repo-root>/.quickbase/quickbase.jsonc",
        ));
}

#[test]
fn status_help_lists_status_options() {
    let mut command = Command::cargo_bin("quickbase").expect("binary exists");

    command
        .args(["util", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--text"))
        .stdout(predicate::str::contains("--markdown"))
        .stdout(predicate::str::contains("--base-url"))
        .stdout(predicate::str::contains("--realm"))
        .stdout(predicate::str::contains("--appId"));
}

#[test]
fn status_network_failure_prints_structured_failure() {
    let repo = TestRepo::new();
    repo.write_config(
        r#"{
          "quickbaseRealm": "example.quickbase.com",
          "quickbaseAppId": "app_failure",
          "quickbaseUserToken": "secret-token",
          "mode": "dryrun"
        }"#,
    );

    repo.command()
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .args([
            "util",
            "status",
            "--json",
            "--base-url",
            "http://127.0.0.1:9",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            repo.config_path().display().to_string(),
        ))
        .stdout(predicate::str::contains(
            "\"quickbaseRealm\": \"example.quickbase.com\"",
        ))
        .stdout(predicate::str::contains("\"target\": \"mock\""))
        .stdout(predicate::str::contains("\"statusCode\": 0"))
        .stdout(predicate::str::contains("\"statusMessage\""))
        .stdout(predicate::str::contains("secret-token").not());
}

#[test]
fn status_invalid_base_url_reports_quickbase_target() {
    let repo = TestRepo::new();
    repo.write_config(
        r#"{
          "quickbaseRealm": "example.quickbase.com",
          "quickbaseAppId": "app_failure",
          "quickbaseUserToken": "secret-token",
          "mode": "live"
        }"#,
    );

    repo.command()
        .args(["util", "status", "--json", "--base-url", "not-a-valid-url"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"target\": \"quickbase\""))
        .stdout(predicate::str::contains("secret-token").not());
}

#[test]
fn make_skill_cli_shape_is_implemented() {
    for args in [
        ["util", "make-skill", "codex"],
        ["util", "make-skill", "claude"],
    ] {
        let repo = TestRepo::new();

        repo.command()
            .args(args)
            .assert()
            .success()
            .stdout(predicate::str::contains("quickbase-api"))
            .stdout(predicate::str::contains("quickbase-cli"));
    }
}
