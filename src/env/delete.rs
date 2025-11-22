use crate::csmrc::Config;
use crate::env::{CommonArgs, env_name};
use crate::micromamba::Micromamba;

use log::{error, info};
use std::fs::remove_dir_all;
use std::process::ExitCode;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Common environment arguments
    #[command(flatten)]
    pub common: CommonArgs,
    /// Avoid prompting for confirmation
    #[arg(short, long, action)]
    pub force: bool,
}

pub fn run(micromamba: Micromamba, args: Args, config: &Config) -> Result<(), ExitCode> {
    let env_name = env_name(args.common.name, &args.common.env_file)?;
    let Some(env_path) = micromamba.path_for_env(&env_name) else {
        error!("Could not determine path for environment '{}'", env_name);
        return Err(ExitCode::FAILURE);
    };
    if !env_path.exists() {
        error!(
            "The environment '{}' at '{}' does not exist",
            env_name,
            env_path.display()
        );
        return Err(ExitCode::FAILURE);
    }
    if config.noop_mode {
        info!(
            "Would delete the environment '{}' at '{}'",
            env_name,
            env_path.display()
        );
        return Ok(());
    }
    info!(
        "Deleting environment '{}' at '{}'",
        env_name,
        env_path.display()
    );

    if !args.force {
        eprint!("Continue? [y/N]: ");
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            error!("Failed to get user input");
            return Err(ExitCode::FAILURE);
        }
        if !(["y", "Y", "yes"].contains(&input.trim())) {
            error!("Exiting.");
            return Err(ExitCode::FAILURE);
        }
    }

    match remove_dir_all(env_path) {
        Err(e) => {
            error!("Error removing the environment path: {}", e);
            Err(ExitCode::FAILURE)
        }
        Ok(()) => {
            info!("Done.");
            Ok(())
        }
    }
}
