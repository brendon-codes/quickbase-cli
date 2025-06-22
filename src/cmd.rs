use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::{
    config::{self, ConfigMode},
    error::{QuickbaseCliError, Result},
    output::{OutputFormat, write_serialized},
    quickbase::{
        client::{QuickbaseClient, QuickbaseResponse},
        operation::{Operation, find_operation, operation_count, operations},
        request::{PreparedRequest, RequestInput, prepare_request},
    },
};

#[derive(Debug)]
pub struct CmdOptions {
    pub output: OutputFormat,
    pub base_url: Option<String>,
    pub realm: Option<String>,
    pub raw_args: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CmdOutput {
    operation_id: String,
    requested_operation_id: String,
    mode: &'static str,
    dry_run: bool,
    request: PreparedRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<QuickbaseResponse>,
}

#[derive(Debug)]
struct ParsedArgs {
    operation_id: String,
    output: OutputFormat,
    base_url: Option<String>,
    realm: Option<String>,
    params: BTreeMap<String, String>,
    body: Option<Value>,
}

pub async fn execute(options: CmdOptions) -> Result<()> {
    if should_print_top_level_help(&options.raw_args) {
        print_cmd_help();
        return Ok(());
    }

    let mut parsed = parse_args(options)?;
    let operation = find_operation(&parsed.operation_id)
        .ok_or_else(|| command_error(format!("unknown operation ID `{}`", parsed.operation_id)))?;

    if operation_help_requested(&parsed) {
        print_operation_help(operation);
        return Ok(());
    }

    let config = config::load_default_config()?;
    inject_config_app_id(operation, &config.app_id, &mut parsed.params);
    let request = prepare_request(
        operation,
        &config,
        RequestInput {
            base_url: parsed.base_url,
            realm: parsed.realm,
            args: parsed.params,
            body: parsed.body,
        },
    )?;

    if config.mode == ConfigMode::Dryrun {
        write_serialized(
            parsed.output,
            &CmdOutput {
                operation_id: operation.operation_id.clone(),
                requested_operation_id: parsed.operation_id,
                mode: config.mode.as_str(),
                dry_run: true,
                request: request.redacted(),
                response: None,
            },
        )?;
        return Ok(());
    }

    let response = QuickbaseClient::new().execute(&request).await?;
    let success = response.success;
    let status = response.status;
    write_serialized(
        parsed.output,
        &CmdOutput {
            operation_id: operation.operation_id.clone(),
            requested_operation_id: parsed.operation_id,
            mode: config.mode.as_str(),
            dry_run: false,
            request: request.redacted(),
            response: Some(response),
        },
    )?;

    if success {
        Ok(())
    } else {
        Err(QuickbaseCliError::HttpStatus { status })
    }
}

fn parse_args(options: CmdOptions) -> Result<ParsedArgs> {
    let Some((operation_id, tail)) = options.raw_args.split_first() else {
        return Err(command_error("missing operation ID after `cmd`"));
    };

    if operation_id.starts_with('-') {
        return Err(command_error("missing operation ID after `cmd`"));
    }

    let mut parsed = ParsedArgs {
        operation_id: operation_id.clone(),
        output: options.output,
        base_url: options.base_url,
        realm: options.realm,
        params: BTreeMap::new(),
        body: None,
    };
    let operation_accepts_realm_param = find_operation(operation_id)
        .map(|operation| {
            operation
                .query_params
                .iter()
                .any(|parameter| parameter.name == "realm")
                || operation
                    .path_params
                    .iter()
                    .any(|parameter| parameter.name == "realm")
        })
        .unwrap_or(false);

    let mut index = 0;
    while index < tail.len() {
        let arg = &tail[index];
        if arg == "--help" || arg == "-h" {
            parsed.params.insert("help".to_owned(), "true".to_owned());
            index += 1;
            continue;
        }

        if !arg.starts_with("--") {
            return Err(command_error(format!(
                "unexpected positional argument `{arg}`"
            )));
        }

        let without_prefix = &arg[2..];
        let (name, value) = if let Some((name, value)) = without_prefix.split_once('=') {
            (name.to_owned(), value.to_owned())
        } else {
            let name = without_prefix.to_owned();
            if matches!(name.as_str(), "json" | "markdown" | "text") {
                (name, String::new())
            } else {
                index += 1;
                let Some(value) = tail.get(index) else {
                    return Err(command_error(format!("--{name} requires a value")));
                };
                if value.starts_with("--") {
                    return Err(command_error(format!("--{name} requires a value")));
                }
                (name, value.clone())
            }
        };

        match name.as_str() {
            "json" => parsed.output = OutputFormat::Json,
            "markdown" => parsed.output = OutputFormat::Markdown,
            "text" => parsed.output = OutputFormat::Text,
            "base-url" => parsed.base_url = Some(value),
            "realm" if !operation_accepts_realm_param => parsed.realm = Some(value),
            "body" => {
                parsed.body = Some(serde_json::from_str(&value).map_err(|error| {
                    command_error(format!("--body must be valid JSON: {error}"))
                })?);
            }
            _ => {
                parsed.params.insert(name, value);
            }
        }

        index += 1;
    }

    Ok(parsed)
}

fn should_print_top_level_help(args: &[String]) -> bool {
    args.is_empty() || matches!(args, [only] if only == "--help" || only == "-h")
}

fn operation_help_requested(parsed: &ParsedArgs) -> bool {
    parsed
        .params
        .get("help")
        .is_some_and(|value| value == "true")
}

fn inject_config_app_id(
    operation: &Operation,
    app_id: &str,
    params: &mut BTreeMap<String, String>,
) {
    if params.contains_key("appId") {
        return;
    }

    let accepts_app_id = operation
        .path_params
        .iter()
        .chain(operation.query_params.iter())
        .any(|parameter| parameter.name == "appId");

    if accepts_app_id {
        params.insert("appId".to_owned(), app_id.to_owned());
    }
}

pub fn print_cmd_help() {
    println!("Run Quickbase REST API operations by operation ID");
    println!();
    println!("Usage:");
    println!(
        "  quickbase cmd [--json|--markdown|--text] [--base-url URL] [--realm HOST] <operationId> [--param VALUE ...] [--body JSON]"
    );
    println!();
    println!("Options:");
    println!("  --json           Render output as JSON (default)");
    println!("  --markdown       Render output as Markdown");
    println!("  --text           Alias for console-friendly Markdown output");
    println!("  --base-url URL   Override the Quickbase API base URL");
    println!("  --realm HOST     Override the QB-Realm-Hostname header");
    println!("  --body JSON      JSON request body for operations that accept one");
    println!("  --help           Show cmd help or operation-specific help");
    println!();
    println!("Operations ({}):", operation_count());

    let mut current_tag = "";
    for operation in operations() {
        if operation.tag != current_tag {
            current_tag = &operation.tag;
            println!();
            println!("{current_tag}:");
        }
        println!(
            "  {:<34} {} {} - {}",
            operation.operation_id, operation.method, operation.path, operation.summary
        );
    }
}

pub fn print_operation_help(operation: &Operation) {
    println!("{}", operation.operation_id);
    println!();
    println!("{} {}", operation.method, operation.path);
    if !operation.summary.is_empty() {
        println!("{}", operation.summary);
    }
    if !operation.description.is_empty() {
        println!();
        println!("{}", operation.description);
    }
    println!();
    println!("Path arguments:");
    print_params(&operation.path_params);
    println!();
    println!("Query arguments:");
    print_params(&operation.query_params);
    println!();
    println!(
        "Body: {}",
        if operation.has_body {
            "accepts --body JSON"
        } else {
            "not accepted"
        }
    );
    println!();
    println!("Output flags: --json (default), --markdown, --text");
}

fn print_params(params: &[crate::quickbase::operation::Parameter]) {
    if params.is_empty() {
        println!("  none");
        return;
    }

    for parameter in params {
        let required = if parameter.required {
            "required"
        } else {
            "optional"
        };
        println!(
            "  --{:<28} {:<8} {}",
            parameter.name, required, parameter.kind
        );
    }
}

fn command_error(message: impl Into<String>) -> QuickbaseCliError {
    QuickbaseCliError::Command {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prompt_style_body_and_case_insensitive_operation() {
        let parsed = parse_args(CmdOptions {
            output: OutputFormat::Json,
            base_url: None,
            realm: None,
            raw_args: vec![
                "getusers".to_owned(),
                "--text".to_owned(),
                "--accountId=acct".to_owned(),
                "--body".to_owned(),
                r#"{"emails":["a@example.com"],"appIds":["a1"],"nextPageToken":""}"#.to_owned(),
            ],
        })
        .expect("args parse");

        assert_eq!(parsed.operation_id, "getusers");
        assert_eq!(parsed.output, OutputFormat::Text);
        assert_eq!(
            parsed.params.get("accountId").map(String::as_str),
            Some("acct")
        );
        assert!(parsed.body.is_some());
    }

    #[test]
    fn realm_after_generate_document_remains_operation_arg() {
        let parsed = parse_args(CmdOptions {
            output: OutputFormat::Json,
            base_url: None,
            realm: None,
            raw_args: vec![
                "generateDocument".to_owned(),
                "--templateId=1".to_owned(),
                "--tableId=tbl".to_owned(),
                "--filename=doc".to_owned(),
                "--realm=query.realm".to_owned(),
            ],
        })
        .expect("args parse");

        assert_eq!(
            parsed.params.get("realm").map(String::as_str),
            Some("query.realm")
        );
        assert_eq!(parsed.realm, None);
    }

    #[test]
    fn config_app_id_is_injected_only_when_operation_accepts_app_id() {
        let operation = find_operation("createTable").expect("operation exists");
        let mut params = BTreeMap::new();

        inject_config_app_id(operation, "app_config", &mut params);

        assert_eq!(params.get("appId").map(String::as_str), Some("app_config"));

        let operation = find_operation("getUsers").expect("operation exists");
        inject_config_app_id(operation, "app_config", &mut params);

        assert_eq!(params.len(), 1);
    }

    #[test]
    fn explicit_app_id_is_preserved() {
        let operation = find_operation("createTable").expect("operation exists");
        let mut params = BTreeMap::from([("appId".to_owned(), "app_explicit".to_owned())]);

        inject_config_app_id(operation, "app_config", &mut params);

        assert_eq!(
            params.get("appId").map(String::as_str),
            Some("app_explicit")
        );
    }
}
