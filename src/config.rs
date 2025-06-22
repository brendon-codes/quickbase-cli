use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use jsonc_parser::{JsonObject, JsonValue, parse_to_value};
use serde::Serialize;

use crate::error::{QuickbaseCliError, Result};

pub const DEFAULT_CONFIG_PATH: &str = "<repo-root>/.quickbase/quickbase.jsonc";
pub const EXAMPLE_CONFIG_PATH: &str = "examples/.quickbase/quickbase.jsonc";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Config {
    #[serde(rename = "quickbaseAppId")]
    pub app_id: String,
    #[serde(rename = "quickbaseRealm")]
    pub quickbase_realm: String,
    #[serde(rename = "quickbaseUserToken")]
    pub quickbase_user_token: String,
    pub mode: ConfigMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigMode {
    Live,
    Dryrun,
}

impl ConfigMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Dryrun => "dryrun",
        }
    }
}

pub fn default_config_path() -> Result<PathBuf> {
    Ok(repo_root()?.join(".quickbase").join("quickbase.jsonc"))
}

pub fn repo_root() -> Result<PathBuf> {
    let current_dir = env::current_dir().map_err(|error| QuickbaseCliError::Config {
        message: format!("failed to resolve current directory: {error}"),
    })?;
    repo_root_from(&current_dir)
}

pub fn repo_root_from(start: &Path) -> Result<PathBuf> {
    for candidate in start.ancestors() {
        let git_marker = candidate.join(".git");
        if git_marker.is_dir() || git_marker.is_file() {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(QuickbaseCliError::Config {
        message: format!(
            "must be run inside a Git work tree so {DEFAULT_CONFIG_PATH} can be resolved"
        ),
    })
}

pub fn load_default_config() -> Result<Config> {
    load_config_from_path(default_config_path()?)
}

pub fn load_config_from_path(path: impl AsRef<Path>) -> Result<Config> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    parse_config(&text)
}

pub fn parse_config(text: &str) -> Result<Config> {
    let value =
        parse_to_value(text, &Default::default()).map_err(|error| QuickbaseCliError::Config {
            message: format!("failed to parse JSONC: {error}"),
        })?;
    let Some(value) = value else {
        return Err(config_error("config file is empty"));
    };

    let JsonValue::Object(object) = value else {
        return Err(config_error("config root must be an object"));
    };

    let app_id = required_string(&object, "quickbaseAppId")?;
    let quickbase_realm = required_string(&object, "quickbaseRealm")?;
    let quickbase_user_token = required_string(&object, "quickbaseUserToken")?;
    let mode = parse_mode(&required_string(&object, "mode")?)?;

    validate_non_empty(&app_id, "quickbaseAppId")?;
    validate_realm(&quickbase_realm)?;
    validate_non_empty(&quickbase_user_token, "quickbaseUserToken")?;

    Ok(Config {
        app_id,
        quickbase_realm,
        quickbase_user_token,
        mode,
    })
}

fn required_string(object: &JsonObject<'_>, field: &str) -> Result<String> {
    let Some(value) = object.get(field) else {
        return Err(config_error(format!("required field `{field}` is missing")));
    };

    let JsonValue::String(value) = value else {
        return Err(config_error(format!(
            "required field `{field}` must be a string"
        )));
    };

    let value = value.trim().to_owned();
    validate_non_empty(&value, field)?;
    Ok(value)
}

fn parse_mode(value: &str) -> Result<ConfigMode> {
    match value {
        "live" => Ok(ConfigMode::Live),
        "dryrun" => Ok(ConfigMode::Dryrun),
        other => Err(config_error(format!(
            "field `mode` must be `live` or `dryrun`; got `{other}`"
        ))),
    }
}

fn validate_realm(value: &str) -> Result<()> {
    if value.contains("://") {
        return Err(config_error(
            "field `quickbaseRealm` must be a hostname like `example.quickbase.com`, not a full URL",
        ));
    }

    if value.contains('/') || value.chars().any(char::is_whitespace) {
        return Err(config_error(
            "field `quickbaseRealm` must be a hostname without paths or whitespace",
        ));
    }

    if !value.contains('.') || value.split('.').any(str::is_empty) {
        return Err(config_error(
            "field `quickbaseRealm` must be a hostname like `example.quickbase.com`",
        ));
    }

    let valid = value
        .split('.')
        .all(|label| label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    if !valid {
        return Err(config_error(
            "field `quickbaseRealm` may only contain ASCII letters, numbers, hyphens, and dots",
        ));
    }

    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(config_error(format!(
            "required field `{field}` must not be empty"
        )));
    }

    Ok(())
}

fn config_error(message: impl Into<String>) -> QuickbaseCliError {
    QuickbaseCliError::Config {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_from_finds_git_directory_at_start() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        fs::create_dir(temp.path().join(".git")).expect("git marker");

        assert_eq!(
            repo_root_from(temp.path()).expect("repo root"),
            temp.path().to_path_buf()
        );
    }

    #[test]
    fn repo_root_from_walks_up_from_nested_directory() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        fs::create_dir(temp.path().join(".git")).expect("git marker");
        let nested = temp.path().join("a").join("b");
        fs::create_dir_all(&nested).expect("nested dir");

        assert_eq!(
            repo_root_from(&nested).expect("repo root"),
            temp.path().to_path_buf()
        );
    }

    #[test]
    fn repo_root_from_accepts_git_file_for_worktrees() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        fs::write(temp.path().join(".git"), "gitdir: /tmp/worktree").expect("git file");

        assert_eq!(
            repo_root_from(temp.path()).expect("repo root"),
            temp.path().to_path_buf()
        );
    }

    #[test]
    fn repo_root_from_fails_without_git_marker() {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let error = repo_root_from(temp.path()).expect_err("no git root");

        assert!(
            error
                .to_string()
                .contains("must be run inside a Git work tree")
        );
    }
}
