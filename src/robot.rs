use crate::csmrc::Config;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Create a Robotmk robot
    New(CreateArgs),

    /// Run a Robotmk robot
    Run,
}

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// Directory path at which to create the robot
    path: String,
}

pub fn run(config: Config, subcommand: Subcommand) {
    println!("{:?}", config);
    println!("{:?}", subcommand);
}
