use crate::csmrc::Config;
use crate::env::{CommonArgs, env_name};
use crate::micromamba::micromamba;

use std::process::ExitCode;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub common: CommonArgs,
    /// The command to run
    #[arg(value_name = "COMMAND")]
    pub command: String,
    /// Arguments to pass to the command
    #[arg(value_name = "ARGS")]
    pub arguments: Vec<String>,
}

pub fn run(config: Config, args: Args) -> Result<(), ExitCode> {
    let env_name = env_name(args.common.name, &args.common.env_file)?;
    let mut micromamba_args = vec!["run", "--name", &env_name, &args.command];
    micromamba_args.extend(args.arguments.iter().map(|s| s.as_str()));
    micromamba(&config, micromamba_args, true).into()
}
