mod env;
mod robot;

use clap::{Parser, Subcommand};

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
    let cli = Cli::parse();
    match cli.command {
        Command::Env(sub) => env::run(sub),
        Command::Robot(sub) => robot::run(sub),
    }
}
