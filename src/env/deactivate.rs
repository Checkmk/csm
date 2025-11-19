use crate::shell::SupportedShell;

use log::error;
use std::process::ExitCode;

pub fn run() -> Result<(), ExitCode> {
    let Some(shell) = SupportedShell::from_csm_hook() else {
        error!("Your shell does not appear to have the csm hook enabled");
        error!("See 'csm init' for information on how to set up the hook");
        return Err(ExitCode::FAILURE);
    };
    println!("{}", shell.restore_and_unset_env_var("PATH"));
    println!("{}", shell.restore_and_unset_env_var("CONDA_DEFAULT_ENV"));
    println!("{}", shell.restore_and_unset_env_var("CONDA_PREFIX"));
    println!("{}", shell.restore_and_unset_env_var("CONDA_SHLVL"));
    Ok(())
}
