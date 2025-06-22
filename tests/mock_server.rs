use std::{fs, path::Path};

use quickbase_cli::{
    mock::server::{BoundMockServer, MockServerOptions},
    quickbase::operation::operations,
};
use reqwest::Method;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::oneshot;

struct TestServer {
    base_url: String,
    shutdown: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<quickbase_cli::Result<()>>,
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

#[tokio::test]
async fn server_starts_and_health_reports_registry_count() {
    let temp = TempDir::new().expect("temp dir");
    let server = TestServer::start(temp.path()).await;

    let body: Value = reqwest::get(format!("{}/health", server.base_url))
        .await
        .expect("health request succeeds")
        .json()
        .await
        .expect("health response is JSON");

    assert_eq!(body.get("ok").and_then(Value::as_bool), Some(true));
    assert_eq!(body.get("operationCount").and_then(Value::as_u64), Some(67));

    server.stop().await;
}

#[tokio::test]
async fn startup_and_shutdown_reset_managed_data() {
    let temp = TempDir::new().expect("temp dir");
    let stale_realm = temp.path().join("realms").join("stale");
    fs::create_dir_all(&stale_realm).expect("stale realm dir");
    fs::write(stale_realm.join("junk.json"), "{}").expect("stale file");
    fs::write(temp.path().join("state.json"), "{}").expect("stale state");

    let server = TestServer::start(temp.path()).await;
    assert!(!stale_realm.join("junk.json").exists());
    assert!(temp.path().join("state.json").exists());

    let client = reqwest::Client::new();
    let response: Value = client
        .post(format!("{}/apps", server.base_url))
        .headers(mock_headers())
        .json(&json!({ "name": "Reset App" }))
        .send()
        .await
        .expect("create app request")
        .json()
        .await
        .expect("create app JSON");
    let app_id = response.get("id").and_then(Value::as_str).expect("app id");
    assert!(
        temp.path()
            .join("realms/test.quickbase.com/apps")
            .join(app_id)
            .join("app.json")
            .exists()
    );

    server.stop().await;

    assert!(!temp.path().join("realms/test.quickbase.com/apps").exists());
    assert!(temp.path().join("state.json").exists());
}

#[tokio::test]
async fn app_table_field_and_record_flow_persists_during_run() {
    let temp = TempDir::new().expect("temp dir");
    let server = TestServer::start(temp.path()).await;
    let client = reqwest::Client::new();

    let app: Value = client
        .post(format!("{}/apps", server.base_url))
        .headers(mock_headers())
        .json(&json!({ "name": "Flow App" }))
        .send()
        .await
        .expect("create app")
        .json()
        .await
        .expect("app JSON");
    let app_id = app.get("id").and_then(Value::as_str).expect("app id");

    let table: Value = client
        .post(format!("{}/tables?appId={app_id}", server.base_url))
        .headers(mock_headers())
        .json(&json!({ "name": "Tasks" }))
        .send()
        .await
        .expect("create table")
        .json()
        .await
        .expect("table JSON");
    let table_id = table.get("id").and_then(Value::as_str).expect("table id");

    let field: Value = client
        .post(format!("{}/fields?tableId={table_id}", server.base_url))
        .headers(mock_headers())
        .json(&json!({ "label": "Task Name", "fieldType": "text" }))
        .send()
        .await
        .expect("create field")
        .json()
        .await
        .expect("field JSON");
    assert_eq!(
        field.get("label").and_then(Value::as_str),
        Some("Task Name")
    );

    client
        .post(format!("{}/records", server.base_url))
        .headers(mock_headers())
        .json(&json!({
            "to": table_id,
            "data": [
                { "6": { "value": "Write tests" } }
            ]
        }))
        .send()
        .await
        .expect("upsert records")
        .error_for_status()
        .expect("upsert succeeds");

    let query: Value = client
        .post(format!("{}/records/query", server.base_url))
        .headers(mock_headers())
        .json(&json!({ "from": table_id }))
        .send()
        .await
        .expect("run query")
        .json()
        .await
        .expect("query JSON");
    assert_eq!(
        query
            .pointer("/metadata/numRecords")
            .and_then(Value::as_u64),
        Some(1)
    );

    assert!(
        temp.path()
            .join("realms/test.quickbase.com/apps")
            .join(app_id)
            .join("tables")
            .join(table_id)
            .join("records.json")
            .exists()
    );

    server.stop().await;
}

#[tokio::test]
async fn every_registry_operation_has_a_mock_route() {
    let temp = TempDir::new().expect("temp dir");
    let server = TestServer::start(temp.path()).await;
    let client = reqwest::Client::new();

    for operation in operations() {
        let method = Method::from_bytes(operation.method.as_bytes()).expect("valid method");
        let url = format!("{}{}", server.base_url, sample_path(&operation.path));
        let mut request = client.request(method, url).headers(mock_headers());
        if operation.has_body {
            request = request.json(&json!({}));
        }

        let response = request
            .send()
            .await
            .unwrap_or_else(|error| panic!("{} request sends: {error}", operation.operation_id));
        assert_ne!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "{} should be routed",
            operation.operation_id
        );
        assert!(
            response.status().is_success(),
            "{} should return a deterministic success response, got {}",
            operation.operation_id,
            response.status()
        );
    }

    server.stop().await;
}

#[tokio::test]
async fn required_auth_headers_are_enforced() {
    let temp = TempDir::new().expect("temp dir");
    let server = TestServer::start(temp.path()).await;

    let response = reqwest::get(format!("{}/apps/app_1", server.base_url))
        .await
        .expect("request succeeds");
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    server.stop().await;
}

fn mock_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "QB-Realm-Hostname",
        reqwest::header::HeaderValue::from_static("test.quickbase.com"),
    );
    headers.insert(
        "Authorization",
        reqwest::header::HeaderValue::from_static("QB-USER-TOKEN mock-token"),
    );
    headers
}

fn sample_path(path: &str) -> String {
    let mut output = String::new();
    let mut chars = path.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '{' {
            output.push(character);
            continue;
        }

        for next in chars.by_ref() {
            if next == '}' {
                break;
            }
        }
        output.push('1');
    }
    output
}
