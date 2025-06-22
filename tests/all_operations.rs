use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use assert_cmd::Command;
use predicates::prelude::*;
use quickbase_cli::quickbase::{
    operation::{Operation, operations},
    reference::QUICKBASE_REST_API_YAML,
};
use serde_json::Value;
use tempfile::TempDir;

struct TestRepo {
    _temp: TempDir,
    nested: PathBuf,
    config_dir: PathBuf,
}

impl TestRepo {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp repo");
        let root = temp.path().to_path_buf();
        std::fs::create_dir(root.join(".git")).expect("git marker");
        let nested = root.join("nested").join("workdir");
        std::fs::create_dir_all(&nested).expect("nested workdir");
        let config_dir = root.join(".quickbase");

        Self {
            _temp: temp,
            nested,
            config_dir,
        }
    }

    fn write_config(&self, contents: &str) {
        std::fs::create_dir_all(&self.config_dir).expect("config dir");
        std::fs::write(self.config_dir.join("quickbase.jsonc"), contents).expect("config file");
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("quickbase").expect("binary exists");
        command.current_dir(&self.nested);
        command
    }
}

#[test]
fn reference_operation_ids_match_command_registry() {
    let reference = reference_operations();
    let reference_ids = reference.keys().collect::<Vec<_>>();
    let registry_ids = operations()
        .iter()
        .map(|operation| &operation.operation_id)
        .collect::<Vec<_>>();

    assert_eq!(registry_ids.len(), 67);
    assert_eq!(registry_ids.len(), reference_ids.len());

    for operation in operations() {
        let reference = reference
            .get(&operation.operation_id)
            .unwrap_or_else(|| panic!("{} exists in reference", operation.operation_id));
        assert_eq!(
            (operation.method.as_str(), operation.path.as_str()),
            (reference.method.as_str(), reference.path.as_str()),
            "{} method/path should match reference",
            operation.operation_id
        );
    }
}

#[test]
fn rust_and_skill_reference_artifacts_match() {
    let rust_reference: Value =
        yaml_serde::from_str(QUICKBASE_REST_API_YAML).expect("Rust reference YAML parses");
    let skill_reference: Value = yaml_serde::from_str(include_str!(
        "../.codex/skills/quickbase-api/references/quickbase-rest-api.yaml"
    ))
    .expect("skill reference YAML parses");

    assert_eq!(rust_reference, skill_reference);
}

#[test]
fn registry_operations_have_cmd_help_and_summaries() {
    let mut cmd_help = Command::cargo_bin("quickbase").expect("binary exists");
    let help = cmd_help
        .args(["cmd", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(help).expect("cmd help is utf8");

    for operation in operations() {
        assert!(
            !operation.summary.trim().is_empty(),
            "{} should have a summary",
            operation.operation_id
        );
        assert!(
            help.contains(&operation.operation_id),
            "cmd help should list {}",
            operation.operation_id
        );
        assert_operation_help(operation);
    }
}

#[test]
fn every_operation_can_produce_dry_run_json() {
    let repo = TestRepo::new();
    repo.write_config(
        r#"{
  "quickbaseRealm": "example.quickbase.com",
  "quickbaseAppId": "app_1",
  "quickbaseUserToken": "secret-token",
  "mode": "dryrun"
}
"#,
    );

    for operation in operations() {
        let output = run_dry_run(&repo, operation);
        assert_eq!(
            output.get("operationId").and_then(Value::as_str),
            Some(operation.operation_id.as_str()),
            "{} should render its registry operation ID",
            operation.operation_id
        );
        assert_eq!(
            output.get("mode").and_then(Value::as_str),
            Some("dryrun"),
            "{} should use dry-run config mode",
            operation.operation_id
        );
        assert_eq!(
            output.get("dryRun").and_then(Value::as_bool),
            Some(true),
            "{} should not perform network I/O",
            operation.operation_id
        );
        assert_eq!(
            output.pointer("/request/method").and_then(Value::as_str),
            Some(operation.method.as_str()),
            "{} should use registry method",
            operation.operation_id
        );
        assert!(
            output
                .pointer("/request/path")
                .and_then(Value::as_str)
                .is_some_and(|path| !path.contains('{') && !path.contains('}')),
            "{} should expand path placeholders",
            operation.operation_id
        );
        assert_eq!(
            output
                .pointer("/request/headers/Authorization")
                .and_then(Value::as_str),
            operation
                .requires_auth
                .then_some("QB-USER-TOKEN [REDACTED]"),
            "{} should redact auth consistently",
            operation.operation_id
        );
        if operation.has_body {
            assert!(
                output
                    .get("request")
                    .and_then(|request| request.get("body"))
                    .is_some(),
                "{} should include the supplied body fixture",
                operation.operation_id
            );
        }
    }
}

fn assert_operation_help(operation: &Operation) {
    let mut command = Command::cargo_bin("quickbase").expect("binary exists");
    command
        .args(["cmd", operation.operation_id.as_str(), "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&operation.operation_id))
        .stdout(predicate::str::contains(format!(
            "{} {}",
            operation.method, operation.path
        )))
        .stdout(predicate::str::contains(&operation.summary));
}

fn run_dry_run(repo: &TestRepo, operation: &Operation) -> Value {
    let mut command = repo.command();
    command.timeout(Duration::from_secs(10)).args([
        "cmd",
        "--json",
        operation.operation_id.as_str(),
    ]);

    for (name, value) in sample_args(operation) {
        command.arg(format!("--{name}={value}"));
    }
    if operation.has_body {
        command.args(["--body", "{}"]);
    }

    let output = command.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&output).unwrap_or_else(|error| {
        panic!(
            "{} should output JSON: {error}\n{}",
            operation.operation_id,
            String::from_utf8_lossy(&output)
        )
    })
}

fn sample_args(operation: &Operation) -> BTreeMap<String, String> {
    operation
        .path_params
        .iter()
        .chain(operation.query_params.iter())
        .filter(|parameter| parameter.required)
        .map(|parameter| {
            (
                parameter.name.clone(),
                sample_value(parameter.name.as_str(), parameter.kind.as_str()),
            )
        })
        .collect()
}

fn sample_value(name: &str, kind: &str) -> String {
    match kind {
        "boolean" => "true".to_owned(),
        "integer" | "int" => "1".to_owned(),
        "number" => "1.5".to_owned(),
        _ if name.eq_ignore_ascii_case("day") => "2026-06-09".to_owned(),
        _ if name.eq_ignore_ascii_case("appId") => "app_1".to_owned(),
        _ if name.eq_ignore_ascii_case("tableId") => "table_1".to_owned(),
        _ if name.eq_ignore_ascii_case("recordId") => "1".to_owned(),
        _ if name.eq_ignore_ascii_case("fieldId") => "6".to_owned(),
        _ if name.eq_ignore_ascii_case("gid") => "1".to_owned(),
        _ if name.eq_ignore_ascii_case("templateId") => "1".to_owned(),
        _ if name.eq_ignore_ascii_case("versionNumber") => "1".to_owned(),
        _ if name.eq_ignore_ascii_case("dbid") => "app_1".to_owned(),
        _ if name.eq_ignore_ascii_case("realm") => "example.quickbase.com".to_owned(),
        _ if name.eq_ignore_ascii_case("filename") => "fixture.json".to_owned(),
        _ => "sample".to_owned(),
    }
}

#[derive(Debug)]
struct ReferenceOperation {
    method: String,
    path: String,
}

fn reference_operations() -> BTreeMap<String, ReferenceOperation> {
    let reference: Value =
        yaml_serde::from_str(QUICKBASE_REST_API_YAML).expect("reference YAML parses");
    let paths = reference
        .get("paths")
        .and_then(Value::as_object)
        .expect("reference has paths");
    let methods = ["get", "post", "put", "delete", "patch"];
    let mut output = BTreeMap::new();

    for (path, path_item) in paths {
        for method in methods {
            let Some(operation) = path_item.get(method) else {
                continue;
            };
            let operation_id = operation
                .get("operationId")
                .and_then(Value::as_str)
                .unwrap_or_else(|| panic!("{method} {path} has operationId"));
            output.insert(
                operation_id.to_owned(),
                ReferenceOperation {
                    method: method.to_ascii_uppercase(),
                    path: path.clone(),
                },
            );
        }
    }

    output
}
