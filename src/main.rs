mod env;
mod robot;

use clap::{Parser, Subcommand};
use log::debug;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version)]
/// Checkmk synthetic monitoring command-line tool
struct Cli {
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

fn main() {
    env_logger::init();
    let _ = create_mambarc();
    let cli = Cli::parse();
    match cli.command {
        Command::Env(sub) => env::run(sub),
        Command::Robot(sub) => robot::run(sub),
    }
}

fn create_mambarc() -> std::io::Result<()> {
    let mambarc = r#"
# Show the active environment in the shell prompt
changeps1: True

# proxy_servers:
#   http: http://user:pass@corp.com:8080_
#   https: https://user:pass@corp.com:8080_

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
