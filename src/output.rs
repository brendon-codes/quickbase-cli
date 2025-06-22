use std::fmt;

use anyhow::Context;
use serde::Serialize;

use crate::error::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Json,
    Markdown,
    Text,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json => formatter.write_str("json"),
            Self::Markdown => formatter.write_str("markdown"),
            Self::Text => formatter.write_str("text"),
        }
    }
}

pub fn write_serialized(format: OutputFormat, value: &impl Serialize) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).context("failed to render JSON output")?
            );
        }
        OutputFormat::Markdown | OutputFormat::Text => {
            let value = serde_json::to_value(value).context("failed to render console output")?;
            println!("{}", render_markdown(&value)?);
        }
    }

    Ok(())
}

fn render_markdown(value: &serde_json::Value) -> Result<String> {
    let json = serde_json::to_string_pretty(value).context("failed to render fenced JSON")?;
    Ok(format!("```json\n{json}\n```"))
}
