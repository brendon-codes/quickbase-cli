pub mod cli;
pub mod cmd;
pub mod config;
pub mod error;
pub mod mock;
pub mod output;
pub mod quickbase;
pub mod server;
pub mod skills;
pub mod util;

use clap::Parser;

pub use crate::error::{QuickbaseCliError, Result};

pub async fn run() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.execute().await
}
