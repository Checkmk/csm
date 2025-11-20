use crate::env::{CommonArgs, env_name};
use crate::micromamba::Micromamba;
use crate::shell::SupportedShell;

use log::{error, info};
use std::process::ExitCode;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Common environment arguments
    #[command(flatten)]
    pub common: CommonArgs,
}

pub fn run(micromamba: Micromamba, args: Args) -> Result<(), ExitCode> {
    let Some(shell) = SupportedShell::from_csm_hook() else {
        error!("Your shell does not appear to have the csm hook enabled");
        error!("See 'csm init' for information on how to set up the hook");
        return Err(ExitCode::FAILURE);
    };
    let env_name = env_name(args.common.name, &args.common.env_file)?;
    info!("Activating environment '{}'...", env_name);

    // NOTE: Anything to stdout here is *evaluated by the user's shell*
    // Use the logging macros instead for user-facing output!

    // Start by adding the mamba prefix bin to PATH
    let Some(bin_path) = micromamba.bin_path_for_env(&env_name) else {
        error!(
            "Could not determine binary path for environment '{}'",
            env_name
        );
        return Err(ExitCode::FAILURE);
    };
    println!("{}", shell.prepend_path(&bin_path));

    // And a few conda-specific vars
    let Some(env_path) = micromamba.path_for_env(&env_name) else {
        error!("Could not determine path for environment '{}'", env_name);
        return Err(ExitCode::FAILURE);
    };
    println!("{}", shell.set_env_var("CONDA_DEFAULT_ENV", &env_name));
    println!(
        "{}",
        shell.set_env_var("CONDA_PREFIX", &env_path.to_string_lossy())
    );
    println!("{}", shell.set_env_var("CONDA_SHLVL", "1"));

    Ok(())
}
