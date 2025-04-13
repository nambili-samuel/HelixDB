use clap::{Args, Parser, Subcommand};

pub mod version {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const NAME: &str = "Helix CLI";
    pub const AUTHORS: &str = "Helix Team";
}
use version::{VERSION, NAME, AUTHORS};

#[derive(Debug, Parser)]
#[clap(name = NAME, version = VERSION, author = AUTHORS)]
pub struct HelixCLI {
    #[clap(subcommand)]
    pub command: CommandType,
}

#[derive(Debug, Subcommand)]
pub enum CommandType {
    /// Deploy a Helix project
    Deploy(DeployCommand),

    /// Compile a Helix project
    Compile(CompileCommand),

    /// Lint a Helix project
    Check(LintCommand),

    /// Install the Helix repo
    Install(InstallCommand),

    /// Initialise a new Helix project
    Init(InitCommand),

    /// Test a Helix project
    Test(TestCommand),

    /// List running Helix instances
    Instances(InstancesCommand),

    /// Stop Helix instances
    Stop(StopCommand),

    /// Start a stopped Helix instance
    Start(StartCommand),

    /// Update Helix CLI to the latest version
    Update(UpdateCommand),

    /// SQLite -> Helix
    IngestSqlite(IngestSqliteCommand),
}

#[derive(Debug, Args)]
#[clap(name = "deploy", about = "Deploy a Helix project")]
pub struct DeployCommand {
    #[clap(short, long, help = "The path to the project")]
    pub path: Option<String>,

    #[clap(short, long, help = "The output path")]
    pub output: Option<String>,

    #[clap(short, long, help = "Should build for local machine")]
    pub local: bool,

    #[clap(short, long, help = "Port to run the instance on")]
    pub port: Option<u16>,

    #[clap(short, long, help = "Should generate python bindings")]
    pub gen_py: bool,
}

#[derive(Debug, Args)]
#[clap(name = "compile", about = "Compile a Helix project")]
pub struct CompileCommand {
    #[clap(short, long, help = "The path to the project")]
    pub path: Option<String>,

    #[clap(short, long, help = "The output path")]
    pub output: Option<String>,

    #[clap(short, long, help = "Should generate python bindings")]
    pub gen_py: bool,

    // #[clap(short, long, help = "The target platform")]
    // pub target: Option<String>,

    // #[clap(short, long, help = "Should compile in release mode")]
    // pub release: bool,
}

#[derive(Debug, Args)]
#[clap(name = "check", about = "Lint a Helix project")]
pub struct LintCommand {
    #[clap(short, long, help = "The path to the project")]
    pub path: Option<String>,
}

#[derive(Debug, Args)]
#[clap(name = "install", about = "Install the Helix repo")]
pub struct InstallCommand {
    #[clap(short, long, help = "The path to the project")]
    pub path: Option<String>,
}

#[derive(Debug, Args)]
#[clap(name = "init", about = "Initialise a new Helix project")]
pub struct InitCommand {
    #[clap(short, long, help = "The path to the project")]
    pub path: Option<String>,

    #[clap(short, long, help = "Should initialize for python")]
    pub py: bool,
}

#[derive(Debug, Args)]
#[clap(name = "test", about = "Test a Helix project")]
pub struct TestCommand {
    #[clap(short, long, help = "The path to the project")]
    pub path: Option<String>,

    #[clap(short, long, help = "The test to run")]
    pub test: Option<String>,
}

#[derive(Debug, Args)]
#[clap(name = "instances", about = "List running Helix instances")]
pub struct InstancesCommand {}

#[derive(Debug, Args)]
#[clap(name = "stop", about = "Stop Helix instances")]
pub struct StopCommand {
    #[clap(long, help = "Stop all running instances")]
    pub all: bool,

    #[clap(help = "Instance ID to stop")]
    pub instance_id: Option<String>,
}

#[derive(Debug, Args)]
#[clap(name = "start", about = "Start a stopped Helix instance")]
pub struct StartCommand {
    #[clap(help = "Instance ID to Start")]
    pub instance_id: String,
}

#[derive(Debug, Args)]
#[clap(name = "update", about = "Update Helix CLI to the latest version")]
pub struct UpdateCommand {}

#[derive(Debug, Args)]
#[clap(name = "ingest_sqlite", about = "Migrate relationsal data from sqlite to helix")]
pub struct IngestSqliteCommand {
    #[clap(short, long, required = true, help = "Path to the sqlite.db file")]
    pub path: String,

    #[clap(short, long, required = true, help = "Helixdb instance to ingest data into")]
    pub instance: String,
}

#[derive(Debug)]
pub enum CliError {
    Io(std::io::Error),
    New(String),
    ConfigFileNotFound,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "IO error: {}", e),
            CliError::New(msg) => write!(f, "{}", msg),
            CliError::ConfigFileNotFound => write!(f, "Config file not found"),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

impl From<&'static str> for CliError {
    fn from(e: &'static str) -> Self {
        CliError::New(e.to_string())
    }
}

impl From<String> for CliError {
    fn from(e: String) -> Self {
        CliError::New(e)
    }
}

impl From<sonic_rs::Error> for CliError {
    fn from(e: sonic_rs::Error) -> Self {
        CliError::New(e.to_string())
    }
}
