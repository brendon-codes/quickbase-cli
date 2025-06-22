use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::Duration,
};

use assert_cmd::Command;
use predicates::prelude::*;
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

    fn write_standard_config(&self, mode: &str, token: &str) {
        std::fs::create_dir_all(&self.config_dir).expect("config dir");
        std::fs::write(
            self.config_dir.join("quickbase.jsonc"),
            format!(
                r#"{{
  "quickbaseRealm": "example.quickbase.com",
  "quickbaseAppId": "app_config",
  "quickbaseUserToken": "{token}",
  "mode": "{mode}"
}}
"#
            ),
        )
        .expect("config file");
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("quickbase").expect("binary exists");
        command.current_dir(&self.nested);
        command
    }
}

#[test]
fn cmd_help_lists_registry_operations() {
    let mut command = Command::cargo_bin("quickbase").expect("binary exists");

    command
        .args(["cmd", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Operations (67):"))
        .stdout(predicate::str::contains("createTable"))
        .stdout(predicate::str::contains("--base-url"))
        .stdout(predicate::str::contains("--text"));
}

#[test]
fn operation_help_is_generated_from_registry() {
    let mut command = Command::cargo_bin("quickbase").expect("binary exists");

    command
        .args(["cmd", "createTable", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("createTable"))
        .stdout(predicate::str::contains("POST /tables"))
        .stdout(predicate::str::contains("--appId"))
        .stdout(predicate::str::contains("accepts --body JSON"));
}

#[test]
fn dry_run_outputs_request_without_network_and_redacts_token() {
    let repo = TestRepo::new();
    repo.write_standard_config("dryrun", "secret-token");

    repo.command()
        .args([
            "cmd",
            "--json",
            "createTable",
            "--appId=app123",
            "--body",
            r#"{"name":"My Table"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""operationId": "createTable""#))
        .stdout(predicate::str::contains(r#""mode": "dryrun""#))
        .stdout(predicate::str::contains(r#""dryRun": true"#))
        .stdout(predicate::str::contains(
            r#""Authorization": "QB-USER-TOKEN [REDACTED]""#,
        ))
        .stdout(predicate::str::contains("secret-token").not())
        .stdout(predicate::str::contains(
            "https://api.quickbase.com/v1/tables?appId=app123",
        ));
}

#[test]
fn dry_run_uses_config_app_id_when_operation_app_id_is_omitted() {
    let repo = TestRepo::new();
    repo.write_standard_config("dryrun", "secret-token");

    repo.command()
        .args([
            "cmd",
            "--json",
            "createTable",
            "--body",
            r#"{"name":"My Table"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://api.quickbase.com/v1/tables?appId=app_config",
        ));
}

#[test]
fn explicit_app_id_overrides_config_app_id() {
    let repo = TestRepo::new();
    repo.write_standard_config("dryrun", "secret-token");

    repo.command()
        .args([
            "cmd",
            "--json",
            "createTable",
            "--appId=app_explicit",
            "--body",
            r#"{"name":"My Table"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "https://api.quickbase.com/v1/tables?appId=app_explicit",
        ))
        .stdout(predicate::str::contains("app_config").not());
}

#[test]
fn prompt_example_case_insensitive_operation_parses() {
    let repo = TestRepo::new();
    repo.write_standard_config("dryrun", "secret-token");

    repo.command()
        .args([
            "cmd",
            "--text",
            "getusers",
            "--accountId=123",
            "--body",
            r#"{"emails":["a@example.com"],"appIds":["a1","a2"],"nextPageToken":""}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""operationId": "getUsers""#))
        .stdout(predicate::str::contains(
            r#""requestedOperationId": "getusers""#,
        ));
}

#[test]
fn missing_required_non_app_operation_args_fail() {
    let repo = TestRepo::new();
    repo.write_standard_config("dryrun", "secret-token");

    repo.command()
        .args(["cmd", "getTable"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires --tableId"));
}

#[test]
fn malformed_body_fails_before_request() {
    let repo = TestRepo::new();
    repo.write_standard_config("dryrun", "secret-token");

    repo.command()
        .args(["cmd", "createTable", "--appId=app123", "--body={not-json}"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--body must be valid JSON"));
}

#[test]
fn base_url_routes_live_request_to_local_server() {
    let repo = TestRepo::new();
    repo.write_standard_config("live", "live-token");
    let (base_url, received) = start_one_request_server();

    repo.command()
        .args([
            "cmd",
            "--json",
            "--base-url",
            &base_url,
            "createTable",
            "--appId=app123",
            "--body",
            r#"{"name":"My Table"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""mode": "live""#))
        .stdout(predicate::str::contains(r#""status": 201"#))
        .stdout(predicate::str::contains("QB-USER-TOKEN [REDACTED]"))
        .stdout(predicate::str::contains("live-token").not());

    let request = received
        .recv_timeout(Duration::from_secs(2))
        .expect("server receives request");
    assert!(
        request.starts_with("POST /tables?appId=app123 HTTP/1.1"),
        "{request}"
    );
    assert!(
        request.contains("authorization: QB-USER-TOKEN live-token")
            || request.contains("Authorization: QB-USER-TOKEN live-token"),
        "{request}"
    );
    assert!(request.contains(r#"{"name":"My Table"}"#), "{request}");
}

fn start_one_request_server() -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
    let addr = listener.local_addr().expect("local addr");
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        let request = read_http_request(&mut stream);
        sender.send(request).expect("send request");
        stream
            .write_all(
                b"HTTP/1.1 201 Created\r\ncontent-type: application/json\r\ncontent-length: 12\r\nconnection: close\r\n\r\n{\"ok\":true}\n",
            )
            .expect("write response");
    });

    (format!("http://{addr}"), receiver)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let read = stream.read(&mut chunk).expect("read request");
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if http_request_complete(&buffer) {
            break;
        }
    }

    String::from_utf8(buffer).expect("request is utf8")
}

fn http_request_complete(buffer: &[u8]) -> bool {
    let Some(headers_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&buffer[..headers_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    buffer.len() >= headers_end + 4 + content_length
}
