use std::collections::{BTreeMap, BTreeSet};

use anyhow::Context;
use reqwest::Url;
use serde::Serialize;
use serde_json::Value;

use crate::{
    config::Config,
    error::{QuickbaseCliError, Result},
    quickbase::{API_BASE_URL, operation::Operation},
};

#[derive(Clone, Debug)]
pub struct RequestInput {
    pub base_url: Option<String>,
    pub realm: Option<String>,
    pub args: BTreeMap<String, String>,
    pub body: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedRequest {
    pub operation_id: String,
    pub method: String,
    pub url: String,
    pub path: String,
    pub query: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

impl PreparedRequest {
    pub fn redacted(&self) -> Self {
        let mut redacted = self.clone();
        if redacted.headers.contains_key("Authorization") {
            redacted.headers.insert(
                "Authorization".to_owned(),
                "QB-USER-TOKEN [REDACTED]".to_owned(),
            );
        }
        redacted
    }
}

pub fn prepare_request(
    operation: &Operation,
    config: &Config,
    input: RequestInput,
) -> Result<PreparedRequest> {
    validate_body(operation, &input.body)?;
    validate_args(operation, &input.args)?;

    let path = expand_path(operation, &input.args)?;
    let query = collect_query(operation, &input.args)?;
    let url = build_url(
        input.base_url.as_deref().unwrap_or(API_BASE_URL),
        &path,
        &query,
    )?;
    let headers = build_headers(operation, config, input.realm.as_deref());

    Ok(PreparedRequest {
        operation_id: operation.operation_id.clone(),
        method: operation.method.clone(),
        url,
        path,
        query,
        headers,
        body: input.body,
    })
}

fn validate_body(operation: &Operation, body: &Option<Value>) -> Result<()> {
    if !operation.has_body && body.is_some() {
        return Err(command_error(format!(
            "{} does not accept --body",
            operation.operation_id
        )));
    }

    if operation.body_required && body.is_none() {
        return Err(command_error(format!(
            "{} requires --body JSON",
            operation.operation_id
        )));
    }

    Ok(())
}

fn validate_args(operation: &Operation, args: &BTreeMap<String, String>) -> Result<()> {
    let allowed = operation
        .path_params
        .iter()
        .chain(operation.query_params.iter())
        .map(|parameter| parameter.name.as_str())
        .collect::<BTreeSet<_>>();

    for name in args.keys() {
        if !allowed.contains(name.as_str()) {
            return Err(command_error(format!(
                "unknown argument --{name} for {}",
                operation.operation_id
            )));
        }
    }

    for parameter in operation
        .path_params
        .iter()
        .chain(operation.query_params.iter())
        .filter(|parameter| parameter.required)
    {
        if !args.contains_key(&parameter.name) {
            return Err(command_error(format!(
                "{} requires --{}",
                operation.operation_id, parameter.name
            )));
        }
    }

    for parameter in operation
        .path_params
        .iter()
        .chain(operation.query_params.iter())
    {
        if let Some(value) = args.get(&parameter.name) {
            validate_type(parameter.name.as_str(), parameter.kind.as_str(), value)?;
        }
    }

    Ok(())
}

fn validate_type(name: &str, kind: &str, value: &str) -> Result<()> {
    match kind {
        "boolean" => {
            if !matches!(value, "true" | "false") {
                return Err(command_error(format!("--{name} must be true or false")));
            }
        }
        "integer" | "int" => {
            value
                .parse::<i64>()
                .map(|_| ())
                .map_err(|_| command_error(format!("--{name} must be an integer")))?;
        }
        "number" => {
            value
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| command_error(format!("--{name} must be a number")))?;
        }
        _ => {}
    }

    Ok(())
}

fn expand_path(operation: &Operation, args: &BTreeMap<String, String>) -> Result<String> {
    let mut path = operation.path.clone();
    for parameter in &operation.path_params {
        let value = args.get(&parameter.name).ok_or_else(|| {
            command_error(format!(
                "{} requires --{}",
                operation.operation_id, parameter.name
            ))
        })?;
        path = path.replace(
            &format!("{{{}}}", parameter.name),
            &percent_encode(value.as_bytes()),
        );
    }

    Ok(path)
}

fn collect_query(
    operation: &Operation,
    args: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let query_names = operation
        .query_params
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<BTreeSet<_>>();

    Ok(args
        .iter()
        .filter(|(name, _)| query_names.contains(name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect())
}

fn build_url(base_url: &str, path: &str, query: &BTreeMap<String, String>) -> Result<String> {
    let base_url = base_url.trim_end_matches('/');
    let mut url = Url::parse(&format!("{base_url}{path}"))
        .with_context(|| format!("failed to build request URL from {base_url} and {path}"))?;

    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (name, value) in query {
            pairs.append_pair(name, value);
        }
    }

    Ok(url.to_string())
}

fn build_headers(
    operation: &Operation,
    config: &Config,
    realm_override: Option<&str>,
) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    if operation.requires_realm {
        headers.insert(
            "QB-Realm-Hostname".to_owned(),
            realm_override.unwrap_or(&config.quickbase_realm).to_owned(),
        );
    }
    if operation.requires_auth {
        headers.insert(
            "Authorization".to_owned(),
            format!("QB-USER-TOKEN {}", config.quickbase_user_token),
        );
    }
    headers.insert(
        "User-Agent".to_owned(),
        format!("quickbase-cli/{}", env!("CARGO_PKG_VERSION")),
    );
    if operation.has_body {
        headers.insert("Content-Type".to_owned(), "application/json".to_owned());
    }

    headers
}

fn percent_encode(bytes: &[u8]) -> String {
    let mut encoded = String::new();
    for byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(*byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

fn command_error(message: impl Into<String>) -> QuickbaseCliError {
    QuickbaseCliError::Command {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, ConfigMode},
        quickbase::operation::find_operation,
    };

    #[test]
    fn prepare_request_expands_path_query_and_redacts_token() {
        let operation = find_operation("getTable").expect("operation exists");
        let request = prepare_request(
            operation,
            &test_config(ConfigMode::Dryrun),
            RequestInput {
                base_url: Some("http://localhost:9999".to_owned()),
                realm: None,
                args: BTreeMap::from([
                    ("tableId".to_owned(), "table one".to_owned()),
                    ("appId".to_owned(), "app/one".to_owned()),
                ]),
                body: None,
            },
        )
        .expect("request prepares");

        assert_eq!(request.path, "/tables/table%20one");
        assert_eq!(
            request.url,
            "http://localhost:9999/tables/table%20one?appId=app%2Fone"
        );
        assert_eq!(
            request
                .redacted()
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some("QB-USER-TOKEN [REDACTED]")
        );
    }

    #[test]
    fn unknown_arguments_fail() {
        let operation = find_operation("getApp").expect("operation exists");
        let error = prepare_request(
            operation,
            &test_config(ConfigMode::Dryrun),
            RequestInput {
                base_url: None,
                realm: None,
                args: BTreeMap::from([("bogus".to_owned(), "x".to_owned())]),
                body: None,
            },
        )
        .expect_err("unknown argument fails");

        assert!(error.to_string().contains("unknown argument --bogus"));
    }

    fn test_config(mode: ConfigMode) -> Config {
        Config {
            app_id: "app_1".to_owned(),
            quickbase_realm: "example.quickbase.com".to_owned(),
            quickbase_user_token: "secret-token".to_owned(),
            mode,
        }
    }
}
