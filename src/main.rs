mod csmrc;
mod env;
mod robot;
mod util;

use clap::{Parser, Subcommand};
use log::{LevelFilter, debug, error, warn};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(version)]
/// Checkmk synthetic monitoring command-line tool
struct Cli {
    #[arg(short, long)]
    verbose: bool,

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up logging
    let default_verbosity = if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };
    let mut env_logger_builder = env_logger::Builder::new();
    env_logger_builder.filter_level(default_verbosity);
    env_logger_builder.parse_default_env();
    env_logger_builder.format_timestamp(None);
    env_logger_builder.init();

    let Some(home) = util::homedir() else {
        error!("Failed to determine home directory");
        return ExitCode::FAILURE;
    };

    if let Err(e) = create_mambarc(&home) {
        let attempted_path = home.join(".mambarc");
        warn!("Could not create {}, but continuing: {}", attempted_path.display(), e);
    }

    let config = match csmrc::Config::from_csmrc() {
        Ok(config) => config,
        Err(err) => {
            error!("Failed to parse .csmrc: {}", err);
            return ExitCode::FAILURE;
        }
    };
    match cli.command {
        Command::Env(sub) => env::run(config, sub),
        Command::Robot(sub) => robot::run(config, sub),
    }
}

/// Create a ~/.mambarc (%UserProfile%\.mambarc on Windows) if it does not
/// exist.
fn create_mambarc(home: &PathBuf) -> std::io::Result<()> {
    let mambarc = include_str!("../templates/mambarc");
    let mambarc_path = home.join(".mambarc");
    match File::create_new(&mambarc_path) {
        Ok(mut file) => file.write_all(mambarc.trim_start().as_bytes())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            debug!("File {mambarc_path:?} already exists, not creating")
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
