use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::{self, Config},
    error::{QuickbaseCliError, Result},
    output::OutputFormat,
    quickbase::{
        client::{QuickbaseClient, QuickbaseResponse},
        operation::find_operation,
        request::{RequestInput, prepare_request},
    },
    skills,
};
use anyhow::Context;
use reqwest::Url;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

const EXAMPLE_CONFIG: &str = include_str!("../examples/.quickbase/quickbase.jsonc");
const QUICKBASE_GITIGNORE: &str = "*\n!.gitignore\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillAgent {
    Codex,
    Claude,
}

impl SkillAgent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

pub fn make_config(format: OutputFormat) -> Result<()> {
    let path = config::default_config_path()?;
    let gitignore_path = path
        .parent()
        .ok_or_else(|| QuickbaseCliError::Config {
            message: format!("config path {} has no parent directory", path.display()),
        })?
        .join(".gitignore");
    let already_existed = path.exists();
    let gitignore_already_existed = gitignore_path.exists();

    if !already_existed {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }
        fs::write(&path, EXAMPLE_CONFIG)
            .with_context(|| format!("failed to write config at {}", path.display()))?;
    }

    if !gitignore_already_existed {
        fs::write(&gitignore_path, QUICKBASE_GITIGNORE).with_context(|| {
            format!(
                "failed to write config .gitignore at {}",
                gitignore_path.display()
            )
        })?;
    }

    write_output(
        format,
        &MakeConfigOutput {
            path: path_to_string(&path),
            gitignore_path: path_to_string(&gitignore_path),
            created: !already_existed,
            already_existed,
            gitignore_created: !gitignore_already_existed,
            gitignore_already_existed,
        },
    )
}

pub fn validate_config(format: OutputFormat) -> Result<()> {
    let path = config::default_config_path()?;
    let loaded = config::load_config_from_path(&path)?;

    write_output(
        format,
        &ValidateConfigOutput::new(path_to_string(&path), &loaded),
    )
}

#[derive(Debug)]
pub struct StatusOptions {
    pub output: OutputFormat,
    pub base_url: Option<String>,
    pub realm: Option<String>,
    pub app_id: Option<String>,
}

pub async fn status(options: StatusOptions) -> Result<()> {
    let config_path = config::default_config_path()?;
    let config = config::load_config_from_path(&config_path)?;
    let app_id = options.app_id.unwrap_or_else(|| config.app_id.clone());
    let base_url = options.base_url;
    let effective_realm = options
        .realm
        .unwrap_or_else(|| config.quickbase_realm.clone());
    let context = StatusContext {
        config_path: path_to_string(&config_path),
        quickbase_realm: effective_realm.clone(),
        target: status_target(base_url.as_deref()),
    };
    let client = QuickbaseClient::new();

    let app_response = match execute_status_operation(
        &client,
        "getApp",
        &config,
        base_url.clone(),
        Some(effective_realm.clone()),
        BTreeMap::from([("appId".to_owned(), app_id.clone())]),
    )
    .await
    {
        Ok(response) => response,
        Err(error) => {
            write_status_transport_error(options.output, &context, error.to_string())?;
            return Err(error);
        }
    };

    if !app_response.success {
        write_status_http_failure(options.output, &context, &app_id, &app_response)?;
        return Err(QuickbaseCliError::HttpStatus {
            status: app_response.status,
        });
    }

    let tables_response = match execute_status_operation(
        &client,
        "getAppTables",
        &config,
        base_url,
        Some(effective_realm),
        BTreeMap::from([("appId".to_owned(), app_id.clone())]),
    )
    .await
    {
        Ok(response) => response,
        Err(error) => {
            write_status_transport_error(options.output, &context, error.to_string())?;
            return Err(error);
        }
    };

    if !tables_response.success {
        write_status_http_failure(options.output, &context, &app_id, &tables_response)?;
        return Err(QuickbaseCliError::HttpStatus {
            status: tables_response.status,
        });
    }

    write_output(
        options.output,
        &StatusOutput {
            config_path: context.config_path,
            quickbase_realm: context.quickbase_realm,
            target: context.target,
            app_name: app_response
                .body
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            app_id: app_response
                .body
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(&app_id)
                .to_owned(),
            table_count: tables_response.body.as_array().map_or(0, Vec::len),
            status_code: 200,
            status_message: "OK".to_owned(),
        },
    )
}

pub fn make_skill(format: OutputFormat, agent: SkillAgent) -> Result<()> {
    let root = skill_root(agent)?;

    for skill_name in skills::SKILL_NAMES {
        replace_skill_destination(&root.join(skill_name))?;

        for file in skills::FILES
            .iter()
            .filter(|file| file.skill_name == skill_name)
        {
            let path = root.join(file.skill_name).join(file.relative_path);
            let parent = path.parent().ok_or_else(|| QuickbaseCliError::Command {
                message: format!("skill path {} has no parent directory", path.display()),
            })?;
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create skill directory {}", parent.display())
            })?;
            fs::write(&path, file.contents)
                .with_context(|| format!("failed to write skill file {}", path.display()))?;
        }
    }

    write_output(
        format,
        &MakeSkillOutput {
            location: "local",
            agent: agent.as_str(),
            root: path_to_string(&root),
            skills: skills::SKILL_NAMES.to_vec(),
        },
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MakeConfigOutput {
    path: String,
    gitignore_path: String,
    created: bool,
    already_existed: bool,
    gitignore_created: bool,
    gitignore_already_existed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidateConfigOutput {
    path: String,
    valid: bool,
    quickbase_app_id: String,
    quickbase_realm: String,
    mode: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusOutput {
    config_path: String,
    quickbase_realm: String,
    target: &'static str,
    app_name: String,
    app_id: String,
    table_count: usize,
    status_code: u16,
    status_message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MakeSkillOutput {
    location: &'static str,
    agent: &'static str,
    root: String,
    skills: Vec<&'static str>,
}

#[derive(Clone, Debug)]
struct StatusContext {
    config_path: String,
    quickbase_realm: String,
    target: &'static str,
}

impl ValidateConfigOutput {
    fn new(path: String, config: &Config) -> Self {
        Self {
            path,
            valid: true,
            quickbase_app_id: config.app_id.clone(),
            quickbase_realm: config.quickbase_realm.clone(),
            mode: config.mode.as_str(),
        }
    }
}

async fn execute_status_operation(
    client: &QuickbaseClient,
    operation_id: &str,
    config: &Config,
    base_url: Option<String>,
    realm: Option<String>,
    args: BTreeMap<String, String>,
) -> Result<QuickbaseResponse> {
    let operation = find_operation(operation_id).ok_or_else(|| QuickbaseCliError::Command {
        message: format!("unknown operation ID `{operation_id}`"),
    })?;
    let request = prepare_request(
        operation,
        config,
        RequestInput {
            base_url,
            realm,
            args,
            body: None,
        },
    )?;

    client.execute(&request).await
}

fn write_status_transport_error(
    format: OutputFormat,
    context: &StatusContext,
    message: String,
) -> Result<()> {
    write_output(
        format,
        &StatusOutput {
            config_path: context.config_path.clone(),
            quickbase_realm: context.quickbase_realm.clone(),
            target: context.target,
            app_name: String::new(),
            app_id: String::new(),
            table_count: 0,
            status_code: 0,
            status_message: message,
        },
    )
}

fn write_status_http_failure(
    format: OutputFormat,
    context: &StatusContext,
    app_id: &str,
    response: &QuickbaseResponse,
) -> Result<()> {
    write_output(
        format,
        &StatusOutput {
            config_path: context.config_path.clone(),
            quickbase_realm: context.quickbase_realm.clone(),
            target: context.target,
            app_name: response
                .body
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            app_id: response
                .body
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(app_id)
                .to_owned(),
            table_count: 0,
            status_code: response.status,
            status_message: http_status_message(response.status),
        },
    )
}

fn status_target(base_url: Option<&str>) -> &'static str {
    let Some(base_url) = base_url else {
        return "quickbase";
    };

    let Ok(url) = Url::parse(base_url) else {
        return "quickbase";
    };

    match url.host_str() {
        Some("localhost" | "127.0.0.1" | "::1" | "[::1]") => "mock",
        _ => "quickbase",
    }
}

fn http_status_message(status: u16) -> String {
    reqwest::StatusCode::from_u16(status)
        .ok()
        .and_then(|status| status.canonical_reason().map(str::to_owned))
        .unwrap_or_else(|| "HTTP error".to_owned())
}

fn skill_root(agent: SkillAgent) -> Result<PathBuf> {
    let repo_root = config::repo_root()?;
    Ok(match agent {
        SkillAgent::Codex => repo_root.join(".codex").join("skills"),
        SkillAgent::Claude => repo_root.join(".claude").join("skills"),
    })
}

fn replace_skill_destination(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove skill directory {}", path.display()))?,
        Ok(_) => fs::remove_file(path)
            .with_context(|| format!("failed to remove skill path {}", path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect skill path {}", path.display()))?,
    }

    Ok(())
}

fn write_output(format: OutputFormat, value: &impl Serialize) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).context("failed to render JSON output")?
            );
        }
        OutputFormat::Markdown => {
            let value = serde_json::to_value(value).context("failed to render Markdown output")?;
            println!("{}", markdown_table(&value));
        }
        OutputFormat::Text => {
            let value = serde_json::to_value(value).context("failed to render text output")?;
            println!("{}", markdown_table(&value));
        }
    }

    Ok(())
}

fn markdown_table(value: &serde_json::Value) -> String {
    let object = value.as_object().cloned().unwrap_or_default();
    let rows = object
        .into_iter()
        .map(|(key, value)| {
            let rendered = match value {
                serde_json::Value::String(value) => value,
                other => other.to_string(),
            };
            format!("| {key} | {rendered} |")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("| Field | Value |\n| --- | --- |\n{rows}")
}

fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn skill_agent_enum_renders_cli_names() {
        assert_eq!(SkillAgent::Codex.as_str(), "codex");
        assert_eq!(SkillAgent::Claude.as_str(), "claude");
    }

    #[test]
    fn status_target_detects_local_mock_urls() {
        assert_eq!(status_target(None), "quickbase");
        assert_eq!(
            status_target(Some("https://api.quickbase.com/v1")),
            "quickbase"
        );
        assert_eq!(status_target(Some("http://localhost:3000")), "mock");
        assert_eq!(status_target(Some("http://127.0.0.1:3000")), "mock");
        assert_eq!(status_target(Some("http://[::1]:3000")), "mock");
    }

    #[test]
    fn markdown_output_uses_public_fields() {
        let value = json!({
            "path": "/tmp/config",
            "created": true,
            "alreadyExisted": false
        });

        assert!(markdown_table(&value).contains("| created | true |"));
    }
}
