use crate::env::{CommonArgs, env_name};
use crate::micromamba::Micromamba;

use log::{debug, error};
use std::process::ExitCode;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Common environment arguments
    #[command(flatten)]
    pub common: CommonArgs,
    /// Output path/filename of the packed environment, ending in .tar.gz. If
    /// not specified, the same method is used to determine the environment name
    /// as for the "--name" parameter, and the default name is <env_name>.tar.gz
    #[arg(long, short, value_name = "OUTPUT")]
    pub output: Option<String>,
}

pub fn run(micromamba: Micromamba, args: Args) -> Result<(), ExitCode> {
    let env_name = env_name(args.common.name, &args.common.env_file)?;
    let Some(bin_path) = micromamba.bin_path_for_env(&env_name) else {
        error!(
            "Could not determine binary path for environment '{}'",
            &env_name
        );
        return Err(ExitCode::FAILURE);
    };
    let binary_name = if cfg!(windows) {
        "conda-pack.exe"
    } else {
        "conda-pack"
    };
    let conda_pack = bin_path.join(binary_name);
    if !conda_pack.exists() {
        debug!("Path does not exist: {:?}", conda_pack);
        error!(
            "conda-pack was not found in the environment. It must be installed to use this command."
        );
        return Err(ExitCode::FAILURE);
    }
    let Some(env_path) = micromamba.path_for_env(&env_name) else {
        error!("Could not determine path for environment '{}'", env_name);
        return Err(ExitCode::FAILURE);
    };
    let output = args.output.unwrap_or(format!("{}.tar.gz", env_name));
    micromamba
        .stream(vec![
            "run",
            "--name",
            &env_name,
            &binary_name,
            "--prefix",
            &env_path.to_string_lossy(),
            "--output",
            &output,
        ])
        .into()
}
