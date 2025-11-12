mod csmrc;
mod env;
mod init;
mod micromamba;
mod robot;
mod shell;

use crate::csmrc::Config;
use clap::{CommandFactory, Parser, Subcommand};
use log::{LevelFilter, debug, error, info, warn};
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(version)]
/// Checkmk synthetic monitoring command-line tool
struct Cli {
    /// Enable verbose debugging output
    #[arg(short, long)]
    verbose: bool,

    /// Don't make any changes, only print what would happen
    #[arg(short = 'n', long = "noop")]
    noop_mode: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manipulate Robotmk environments
    #[command(subcommand)]
    Env(env::Subcommand),

    /// Manage Robotmk robots
    #[command(subcommand)]
    Robot(robot::Subcommand),

    /// Initialize shell environment for csm
    Init {
        shell: Option<shell::SupportedShell>,
        /// Generate the actual shell code to be evaluated
        #[arg(long)]
        code: bool,
    },
}

trait CSMResult {
    fn finish(&self) -> ExitCode;
}

impl CSMResult for ExitCode {
    fn finish(&self) -> Self {
        *self
    }
}

impl<T, E: fmt::Display> CSMResult for Result<T, E> {
    fn finish(&self) -> ExitCode {
        match self {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                error!("{}", e);
                ExitCode::FAILURE
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up logging
    let default_verbosity = if cli.verbose {
        LevelFilter::Debug
    } else {
        // We use info level for no-op mode messages.
        LevelFilter::Info
    };
    let mut env_logger_builder = env_logger::Builder::new();
    env_logger_builder.filter_level(default_verbosity);
    env_logger_builder.parse_default_env();
    env_logger_builder.format_timestamp(None);
    env_logger_builder.init();

    let mut config = match Config::from_csmrc() {
        Ok(config) => config,
        Err(err) => {
            error!("Failed to parse .csmrc: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if cli.noop_mode {
        config.noop_mode = true;
    }

    if cli.verbose {
        config.verbose = true;
    }

    let Some(home) = std::env::home_dir() else {
        error!("Failed to determine home directory");
        return ExitCode::FAILURE;
    };

    if let Err(e) = create_mambarc(&config, &home) {
        let attempted_path = home.join(".mambarc");
        warn!(
            "Could not create {}, but continuing: {}",
            attempted_path.display(),
            e
        );
    }

    match cli.command {
        Command::Env(sub) => env::run(config, sub).finish(),
        Command::Robot(sub) => robot::run(config, sub).finish(),
        Command::Init { shell, code } => {
            let mut cmd = Cli::command();
            init::run(config, shell, code, &mut cmd).finish()
        }
    }
}

/// Create a ~/.mambarc (%UserProfile%\.mambarc on Windows) if it does not
/// exist.
fn create_mambarc(config: &Config, home: &Path) -> std::io::Result<()> {
    let mambarc = include_str!("../templates/mambarc");
    let mambarc_path = home.join(".mambarc");

    if config.noop_mode && !mambarc_path.exists() {
        info!("Would create {}", mambarc_path.display());
        return Ok(());
    }

    match File::create_new(&mambarc_path) {
        Ok(mut file) => file.write_all(mambarc.trim_start().as_bytes())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            debug!(
                "File {} already exists, not creating",
                mambarc_path.display()
            )
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
