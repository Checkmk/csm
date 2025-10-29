mod csmrc;
mod env;
mod robot;

use clap::{Parser, Subcommand};
use log::{LevelFilter, debug, error};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

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

fn main() -> Result<(), std::io::Error> {
    let cli = Cli::parse();

    // Set up logging
    let default_verbosity = if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Error
    };
    let mut env_logger_builder = env_logger::Builder::new();
    env_logger_builder.filter_level(default_verbosity);
    env_logger_builder.parse_default_env();
    env_logger_builder.format_timestamp(None);
    env_logger_builder.init();

    let _ = create_mambarc();
    let config = match csmrc::Config::from_csmrc() {
        Ok(config) => config,
        Err(err) => {
            error!("Failed to parse .csmrc: {}", err);
            panic!("Failed to parse .csmrc as valid YAML");
        }
    };
    match cli.command {
        Command::Env(sub) => env::run(config, sub),
        Command::Robot(sub) => robot::run(config, sub),
    }
    Ok(())
}

fn create_mambarc() -> std::io::Result<()> {
    let mambarc = r#"
# Show the active environment in the shell prompt
changeps1: True

# proxy_servers:
#   http: http://user:pass@corp.com:8080
#   https: https://user:pass@corp.com:8080

# Use this only if you are behind a proxy that does SSL inspection
# ssl_verify: false
# ssl_verify: mycorpcert.crt
# ssl_no_revoke: true
"#;
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("Cannot determine home directory");
    let mambarc_path = PathBuf::from(home).join(".mambarc");
    match File::create_new(&mambarc_path) {
        Ok(mut file) => file.write_all(mambarc.trim_start().as_bytes())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            debug!("File {mambarc_path:?} already exists, not creating")
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
