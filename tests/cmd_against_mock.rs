use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use assert_cmd::Command;
use quickbase_cli::mock::server::{BoundMockServer, MockServerOptions};
use serde_json::Value;
use tempfile::TempDir;
use tokio::sync::oneshot;

struct TestRepo {
    _temp: TempDir,
    root: PathBuf,
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
            root,
            nested,
            config_dir,
        }
    }

    fn nested(&self) -> &Path {
        &self.nested
    }

    fn config_path(&self) -> PathBuf {
        self.root.join(".quickbase").join("quickbase.jsonc")
    }

    fn data_dir(&self) -> PathBuf {
        self.root.join(".quickbase").join("data")
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

struct TestServer {
    base_url: String,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<quickbase_cli::Result<()>>,
}

struct CliServer {
    base_url: String,
    data_dir: String,
    child: Child,
}

impl TestServer {
    async fn start(data_dir: &Path) -> Self {
        let bound = BoundMockServer::bind(MockServerOptions {
            host: "127.0.0.1".to_owned(),
            port: 0,
            data_dir: Some(data_dir.to_path_buf()),
        })
        .await
        .expect("server binds");
        let base_url = bound.base_url();
        let (shutdown, receiver) = oneshot::channel();
        let handle = tokio::spawn(bound.serve_until_shutdown(async {
            let _ = receiver.await;
        }));

        Self {
            base_url,
            shutdown: Some(shutdown),
            handle,
        }
    }

    async fn stop(mut self) {
        self.shutdown
            .take()
            .expect("shutdown sender exists")
            .send(())
            .expect("server receives shutdown");
        self.handle
            .await
            .expect("server task joins")
            .expect("server exits cleanly");
    }
}

impl CliServer {
    fn start(data_dir: &Path) -> Self {
        let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin("quickbase"));
        command.args([
            "server",
            "--json",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            "--data-dir",
        ]);
        command.arg(data_dir);
        Self::start_command(command)
    }

    fn start_default(repo: &TestRepo) -> Self {
        let mut command = std::process::Command::new(assert_cmd::cargo::cargo_bin("quickbase"));
        command.current_dir(repo.nested()).args([
            "server",
            "--json",
            "--host",
            "127.0.0.1",
            "--port",
            "0",
        ]);
        Self::start_command(command)
    }

    fn start_command(mut command: std::process::Command) -> Self {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn quickbase server");
        let stdout = child.stdout.take().expect("server stdout is piped");
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let mut startup = String::new();
            for line in BufReader::new(stdout).lines() {
                let line = line.expect("read server stdout");
                startup.push_str(&line);
                startup.push('\n');
                if line == "}" {
                    sender.send(startup).expect("send startup JSON");
                    return;
                }
            }
        });

        let startup = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("server prints startup JSON");
        let value: Value = serde_json::from_str(&startup).expect("startup output is JSON");
        let base_url = value
            .get("baseUrl")
            .and_then(Value::as_str)
            .expect("startup output has baseUrl")
            .to_owned();
        let data_dir = value
            .get("dataDir")
            .and_then(Value::as_str)
            .expect("startup output has dataDir")
            .to_owned();

        Self {
            base_url,
            data_dir,
            child,
        }
    }
}

impl Drop for CliServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cmd_calls_mock_server_with_base_url() {
    let repo = TestRepo::new();
    let data = TempDir::new().expect("data temp dir");
    write_live_config(&repo);
    let server = TestServer::start(data.path()).await;

    let app = run_cmd(
        &repo,
        &server.base_url,
        &["createApp", "--body", r#"{"name":"CLI App"}"#],
    );
    let app_id = app
        .pointer("/response/body/id")
        .and_then(Value::as_str)
        .expect("created app id")
        .to_owned();

    let table = run_cmd(
        &repo,
        &server.base_url,
        &[
            "createTable",
            "--appId",
            &app_id,
            "--body",
            r#"{"name":"CLI Table"}"#,
        ],
    );
    let table_id = table
        .pointer("/response/body/id")
        .and_then(Value::as_str)
        .expect("created table id")
        .to_owned();

    let get_app = run_cmd(&repo, &server.base_url, &["getApp", "--appId", &app_id]);
    assert_eq!(
        get_app.pointer("/response/status").and_then(Value::as_u64),
        Some(200)
    );
    assert_eq!(
        get_app
            .pointer("/response/body/name")
            .and_then(Value::as_str),
        Some("CLI App")
    );

    let commands = representative_commands(&app_id, &table_id);
    for command in commands {
        let output = run_cmd(&repo, &server.base_url, &command);
        assert_eq!(
            output.pointer("/response/status").and_then(Value::as_u64),
            Some(200),
            "{command:?} should succeed against mock server"
        );
        assert_eq!(
            output.pointer("/dryRun").and_then(Value::as_bool),
            Some(false),
            "{command:?} should use live mode"
        );
    }

    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn util_status_uses_config_app_id_against_mock_server() {
    let repo = TestRepo::new();
    let data = TempDir::new().expect("data temp dir");
    write_live_config(&repo);
    let server = TestServer::start(data.path()).await;

    run_cmd(
        &repo,
        &server.base_url,
        &[
            "createApp",
            "--body",
            r#"{"id":"app_config","name":"Configured App"}"#,
        ],
    );
    run_cmd(
        &repo,
        &server.base_url,
        &[
            "createTable",
            "--body",
            r#"{"id":"table_config","name":"Configured Table"}"#,
        ],
    );

    let status = run_status(&repo, &server.base_url, &[]);

    assert_eq!(
        status.get("configPath").and_then(Value::as_str),
        Some(repo.config_path().to_string_lossy().as_ref())
    );
    assert_eq!(
        status.get("quickbaseRealm").and_then(Value::as_str),
        Some("test.quickbase.com")
    );
    assert_eq!(status.get("target").and_then(Value::as_str), Some("mock"));
    assert_eq!(
        status.get("appName").and_then(Value::as_str),
        Some("Configured App")
    );
    assert_eq!(
        status.get("appId").and_then(Value::as_str),
        Some("app_config")
    );
    assert_eq!(status.get("tableCount").and_then(Value::as_u64), Some(1));
    assert_eq!(status.get("statusCode").and_then(Value::as_u64), Some(200));
    assert_eq!(
        status.get("statusMessage").and_then(Value::as_str),
        Some("OK")
    );

    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn util_status_uses_explicit_app_id_against_mock_server() {
    let repo = TestRepo::new();
    let data = TempDir::new().expect("data temp dir");
    write_live_config(&repo);
    let server = TestServer::start(data.path()).await;

    run_cmd(
        &repo,
        &server.base_url,
        &[
            "createApp",
            "--body",
            r#"{"id":"app_explicit","name":"Explicit App"}"#,
        ],
    );

    let status = run_status(&repo, &server.base_url, &["--appId", "app_explicit"]);

    assert_eq!(
        status.get("appName").and_then(Value::as_str),
        Some("Explicit App")
    );
    assert_eq!(
        status.get("appId").and_then(Value::as_str),
        Some("app_explicit")
    );
    assert_eq!(status.get("statusCode").and_then(Value::as_u64), Some(200));

    server.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn util_status_text_and_markdown_include_fields() {
    let repo = TestRepo::new();
    let data = TempDir::new().expect("data temp dir");
    write_live_config(&repo);
    let server = TestServer::start(data.path()).await;

    run_cmd(
        &repo,
        &server.base_url,
        &[
            "createApp",
            "--body",
            r#"{"id":"app_config","name":"Readable App"}"#,
        ],
    );

    for flag in ["--text", "--markdown"] {
        let output = run_status_text(&repo, &server.base_url, flag);
        assert!(
            output.contains(&format!(
                "| configPath | {} |",
                repo.config_path().display()
            )),
            "{output}"
        );
        assert!(
            output.contains("| quickbaseRealm | test.quickbase.com |"),
            "{output}"
        );
        assert!(output.contains("| target | mock |"), "{output}");
        assert!(output.contains("| appName | Readable App |"), "{output}");
        assert!(output.contains("| appId | app_config |"), "{output}");
        assert!(output.contains("| statusCode | 200 |"), "{output}");
    }

    server.stop().await;
}

#[test]
fn cmd_can_use_server_cli_started_on_port_zero() {
    let repo = TestRepo::new();
    let data = TempDir::new().expect("data temp dir");
    write_live_config(&repo);
    let server = CliServer::start(data.path());

    let app = run_cmd(
        &repo,
        &server.base_url,
        &["createApp", "--body", r#"{"name":"Server CLI App"}"#],
    );

    assert_eq!(
        app.pointer("/response/success").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        app.pointer("/response/body/name").and_then(Value::as_str),
        Some("Server CLI App")
    );
}

#[test]
fn server_cli_default_data_dir_uses_repo_root_quickbase_data() {
    let repo = TestRepo::new();
    let server = CliServer::start_default(&repo);

    assert_eq!(server.data_dir, repo.data_dir().to_string_lossy());
}

fn representative_commands(app_id: &str, table_id: &str) -> Vec<Vec<String>> {
    vec![
        strings(&["audit", "--body", r#"{"day":"2026-06-09"}"#]),
        strings(&["getTempTokenDBID", "--dbid", app_id]),
        strings(&[
            "generateDocument",
            "--templateId",
            "1",
            "--tableId",
            table_id,
            "--filename",
            "mock.pdf",
        ]),
        strings(&["getFields", "--tableId", table_id]),
        strings(&[
            "downloadFile",
            "--tableId",
            table_id,
            "--recordId",
            "1",
            "--fieldId",
            "6",
            "--versionNumber",
            "1",
        ]),
        strings(&[
            "runFormula",
            "--body",
            r#"{"formula":"1+1","from":"table_1"}"#,
        ]),
        strings(&[
            "addMembersToGroup",
            "--gid",
            "1",
            "--body",
            r#"{"members":[1]}"#,
        ]),
        strings(&["platformAnalyticReads", "--day", "2026-06-09"]),
        vec![
            "runQuery".to_owned(),
            "--body".to_owned(),
            format!(r#"{{"from":"{table_id}"}}"#),
        ],
        strings(&["getTableReports", "--tableId", table_id]),
        strings(&["createSolution", "--body", r#"{"name":"Mock Solution"}"#]),
        strings(&["getTable", "--tableId", table_id, "--appId", app_id]),
        strings(&["getTrustees", "--appId", app_id]),
        strings(&["getUsers", "--body", r#"{"emails":["user@example.com"]}"#]),
        strings(&["cloneUserToken", "--body", r#"{"name":"Mock Token"}"#]),
    ]
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn run_cmd(repo: &TestRepo, base_url: &str, operation_args: &[impl AsRef<str>]) -> Value {
    let mut command = repo.command();
    command
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .timeout(Duration::from_secs(10))
        .args([
            "cmd",
            "--json",
            "--base-url",
            base_url,
            "--realm",
            "test.quickbase.com",
        ]);
    for arg in operation_args {
        command.arg(arg.as_ref());
    }

    let output = command.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("cmd output is JSON")
}

fn run_status(repo: &TestRepo, base_url: &str, status_args: &[&str]) -> Value {
    let mut command = status_command(repo, base_url);
    command.arg("--json");
    command.args(status_args);

    let output = command.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("status output is JSON")
}

fn run_status_text(repo: &TestRepo, base_url: &str, output_flag: &str) -> String {
    let mut command = status_command(repo, base_url);
    command.arg(output_flag);

    let output = command.assert().success().get_output().stdout.clone();
    String::from_utf8(output).expect("status output is utf8")
}

fn status_command(repo: &TestRepo, base_url: &str) -> Command {
    let mut command = repo.command();
    command
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .timeout(Duration::from_secs(10))
        .args([
            "util",
            "status",
            "--base-url",
            base_url,
            "--realm",
            "test.quickbase.com",
        ]);
    command
}

fn write_live_config(repo: &TestRepo) {
    repo.write_config(
        r#"{
  "quickbaseRealm": "configured.quickbase.com",
  "quickbaseAppId": "app_config",
  "quickbaseUserToken": "mock-token",
  "mode": "live"
}
"#,
    );
}
