use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};

use crate::{
    cmd,
    error::Result,
    output::OutputFormat,
    server,
    util::{self, SkillAgent},
};

#[derive(Debug, Parser)]
#[command(
    name = "quickbase",
    version,
    about = "Query and operate against the Quickbase REST API"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            Commands::Cmd(args) => args.execute().await,
            Commands::Server(args) => args.execute().await,
            Commands::Util(args) => args.execute().await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run generated Quickbase REST API commands.
    Cmd(CmdArgs),
    /// Manage local mock server workflows.
    Server(ServerArgs),
    /// Run helper utilities.
    Util(UtilArgs),
}

#[derive(Debug, Args)]
#[command(disable_help_flag = true)]
pub struct CmdArgs {
    #[command(flatten)]
    output: OutputArgs,

    /// Override the Quickbase API base URL, useful for mock servers and tests.
    #[arg(long)]
    base_url: Option<String>,

    /// Override the QB-Realm-Hostname header from config.
    #[arg(long)]
    realm: Option<String>,

    /// Operation ID followed by operation-specific --param values.
    #[arg(
        value_name = "OPERATION_ARGS",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    operation_args: Vec<String>,
}

impl CmdArgs {
    async fn execute(self) -> Result<()> {
        cmd::execute(cmd::CmdOptions {
            output: self.output.format(),
            base_url: self.base_url,
            realm: self.realm,
            raw_args: self.operation_args,
        })
        .await
    }
}

#[derive(Debug, Args)]
pub struct ServerArgs {
    #[command(flatten)]
    output: OutputArgs,

    /// Host address to bind.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind. Use 0 to request an available port from the OS.
    #[arg(long, default_value_t = 0)]
    port: u16,

    /// Override the mock data root. Defaults to <repo-root>/.quickbase/data.
    #[arg(long)]
    data_dir: Option<PathBuf>,
}

impl ServerArgs {
    async fn execute(self) -> Result<()> {
        server::run(server::ServerOptions {
            output: self.output.format(),
            host: self.host,
            port: self.port,
            data_dir: self.data_dir,
        })
        .await
    }
}

#[derive(Debug, Args)]
pub struct UtilArgs {
    #[command(flatten)]
    output: OutputArgs,

    #[command(subcommand)]
    command: UtilCommand,
}

impl UtilArgs {
    async fn execute(self) -> Result<()> {
        match self.command {
            UtilCommand::MakeConfig(args) => args.execute(self.output.format()),
            UtilCommand::ValidateConfig(args) => args.execute(self.output.format()),
            UtilCommand::MakeSkill(args) => args.execute(self.output.format()),
            UtilCommand::Status(args) => args.execute(self.output.format()).await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum UtilCommand {
    /// Create the default JSONC config file if it does not exist.
    MakeConfig(MakeConfigArgs),
    /// Validate the default JSONC config file.
    ValidateConfig(ValidateConfigArgs),
    /// Contact Quickbase and report app status.
    Status(StatusArgs),
    /// Generate project skill files.
    MakeSkill(MakeSkillArgs),
}

#[derive(Debug, Args)]
struct MakeConfigArgs {
    #[command(flatten)]
    output: OutputArgs,
}

impl MakeConfigArgs {
    fn execute(self, parent_output: OutputFormat) -> Result<()> {
        util::make_config(self.output.format_if_explicit().unwrap_or(parent_output))
    }
}

#[derive(Debug, Args)]
struct ValidateConfigArgs {
    #[command(flatten)]
    output: OutputArgs,
}

impl ValidateConfigArgs {
    fn execute(self, parent_output: OutputFormat) -> Result<()> {
        util::validate_config(self.output.format_if_explicit().unwrap_or(parent_output))
    }
}

#[derive(Debug, Args)]
struct StatusArgs {
    #[command(flatten)]
    output: OutputArgs,

    /// Override the Quickbase API base URL, useful for mock servers and tests.
    #[arg(long)]
    base_url: Option<String>,

    /// Override the QB-Realm-Hostname header from config.
    #[arg(long)]
    realm: Option<String>,

    /// Override the configured appId.
    #[arg(long = "appId")]
    app_id: Option<String>,
}

impl StatusArgs {
    async fn execute(self, parent_output: OutputFormat) -> Result<()> {
        util::status(util::StatusOptions {
            output: self.output.format_if_explicit().unwrap_or(parent_output),
            base_url: self.base_url,
            realm: self.realm,
            app_id: self.app_id,
        })
        .await
    }
}

#[derive(Debug, Args)]
struct MakeSkillArgs {
    #[command(subcommand)]
    agent: MakeSkillAgentCommand,
}

impl MakeSkillArgs {
    fn execute(self, output: OutputFormat) -> Result<()> {
        util::make_skill(output, self.agent.into())
    }
}

#[derive(Debug, Subcommand)]
enum MakeSkillAgentCommand {
    /// Generate project-local Codex skill files.
    Codex,
    /// Generate project-local Claude skill files.
    Claude,
}

impl From<MakeSkillAgentCommand> for SkillAgent {
    fn from(value: MakeSkillAgentCommand) -> Self {
        match value {
            MakeSkillAgentCommand::Codex => Self::Codex,
            MakeSkillAgentCommand::Claude => Self::Claude,
        }
    }
}

#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Render output as JSON.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["markdown", "text"])]
    json: bool,

    /// Render output as Markdown.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["json", "text"])]
    markdown: bool,

    /// Render output as console-friendly text.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["json", "markdown"])]
    text: bool,
}

impl OutputArgs {
    pub(crate) fn format(&self) -> OutputFormat {
        if self.text {
            OutputFormat::Text
        } else if self.markdown {
            OutputFormat::Markdown
        } else if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::Json
        }
    }

    pub(crate) fn format_if_explicit(&self) -> Option<OutputFormat> {
        (self.text || self.markdown || self.json).then(|| self.format())
    }
}
