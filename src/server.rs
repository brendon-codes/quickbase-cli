use std::{
    io::{self, Write},
    path::PathBuf,
};

use anyhow::Context;
use serde::Serialize;

use crate::{
    error::Result,
    mock::{
        DEFAULT_HOST, DEFAULT_PORT,
        server::{BoundMockServer, MockServerOptions},
    },
    output::{OutputFormat, write_serialized},
};

#[derive(Debug)]
pub struct ServerOptions {
    pub output: OutputFormat,
    pub host: String,
    pub port: u16,
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerStarted {
    base_url: String,
    data_dir: PathBuf,
    reset_on_start: bool,
    reset_on_shutdown: bool,
}

pub async fn run(options: ServerOptions) -> Result<()> {
    let bound = BoundMockServer::bind(MockServerOptions {
        host: if options.host.trim().is_empty() {
            DEFAULT_HOST.to_owned()
        } else {
            options.host
        },
        port: options.port,
        data_dir: options.data_dir,
    })
    .await?;

    write_serialized(
        options.output,
        &ServerStarted {
            base_url: bound.base_url(),
            data_dir: bound.data_dir().to_path_buf(),
            reset_on_start: true,
            reset_on_shutdown: true,
        },
    )?;
    io::stdout()
        .flush()
        .context("failed to flush server startup output")?;

    bound
        .serve_until_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            output: OutputFormat::Json,
            host: DEFAULT_HOST.to_owned(),
            port: DEFAULT_PORT,
            data_dir: None,
        }
    }
}
