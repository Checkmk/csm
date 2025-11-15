use crate::csmrc::Config;
use crate::micromamba::{self, MicromambaResult, micromamba};
use crate::shell::SupportedShell;

use log::{debug, error, info};
use serde::Deserialize;
use std::io::{Error, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Create an environment
    Create(CommonEnvArgs),
    /// Activate an environment
    Activate(CommonEnvArgs),
    /// Deactivate an environment
    Deactivate,
    /// Run an executable in an environment
    Run(RunArgs),
    /// Create an archive from an environment. Requires `conda-pack` to be in the environment.
    Pack(PackArgs),
    /// Unpack an archive to create an environment from it.
    Unpack(CommonEnvArgs),
    /// List existing environments
    List,
    /// Display information about the micromamba setup
    Info,
}

#[derive(Debug, clap::Args)]
pub struct CommonEnvArgs {
    /// If specified, the name of the environment. If not specified, csm will
    /// look to robotmk-env.yaml for a "name" field to use instead. As a last
    /// resort, the current directory name will be used
    #[arg(short, long, value_name = "ENV_NAME")]
    name: Option<String>,

    /// If specified, overrides the env file passed to micromamba and used as
    /// a fallback for determining the environment name.
    #[arg(
        long = "env-file",
        value_name = "PATH",
        default_value = "robotmk-env.yaml"
    )]
    env_file: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    #[command(flatten)]
    common: CommonEnvArgs,
    /// The command to run
    #[arg(value_name = "COMMAND")]
    command: String,
    /// Arguments to pass to the command
    #[arg(value_name = "ARGS")]
    arguments: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub struct PackArgs {
    /// Common environment arguments
    #[command(flatten)]
    common: CommonEnvArgs,
    /// Output path/filename of the packed environment. If not specified, the
    /// same method is used to determine the environment name as for the
    /// "--name" parameter, and the default name is <env_name>.tar.gz
    #[arg(long, short, value_name = "OUTPUT")]
    output: Option<String>,
}

/// Contains the fields we need from a parsed `robotmk-env.yml` file.
#[derive(Deserialize)]
struct RobotmkEnv {
    /// The name of the environment
    name: Option<String>,
}

/// Attempt to parse an environment file.
fn parse_env_yaml(path: &Path) -> Result<RobotmkEnv, std::io::Error> {
    let contents = std::fs::read_to_string(path)?;
    serde_yaml_ng::from_str(&contents).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

pub fn determine_env_name(explicit_name: Option<String>, env_yaml_path: &Path) -> Option<String> {
    // If someone gave an explicit --name, use that first.
    if let Some(name) = explicit_name {
        debug!("Using '{}' as env name, given by CLI argument", name);
        return Some(name);
    }

    // Fallback 1: Look for a name key in robotmk-env.yaml
    // We ignore errors from parse_env_yaml() here, we'll fall back
    // below if we can't parse it for some reason
    if let Ok(env) = parse_env_yaml(env_yaml_path)
        && let Some(name) = env.name
    {
        debug!("Using '{}' as env name, found in robotmk-env.yaml", name);
        return Some(name);
    }

    // Fallback 2: Current directory name
    match std::env::current_dir() {
        Err(e) => {
            debug!("Could not determine current directory: {}", e);
            None
        }
        Ok(pathbuf) => match pathbuf.components().next_back() {
            Some(Component::Normal(s)) => match s.to_str().map(String::from) {
                Some(name) => {
                    debug!(
                        "Using '{}' as env name, taken from current directory name",
                        name
                    );
                    Some(name)
                }
                _ => None, // Likely could not convert path name to utf-8
            },
            _ => None, // In theory, I think this should never happen
        },
    }
}

fn env_name(explicit_name: Option<String>, env_yaml_path: &Path) -> Result<String, ExitCode> {
    match determine_env_name(explicit_name, env_yaml_path) {
        Some(name) => Ok(name),
        None => {
            error!("No environment name could be determined. You can specify one with --name");
            Err(ExitCode::FAILURE)
        }
    }
}

pub fn run(config: Config, subcommand: Subcommand) -> Result<(), ExitCode> {
    match subcommand {
        Subcommand::Create(args) => {
            let env_name = env_name(args.name, &args.env_file)?;
            info!(
                "Creating environment '{}' - this may take some time...",
                env_name
            );
            let result = micromamba(
                &config,
                vec![
                    "env",
                    "create",
                    "--file",
                    &args.env_file.to_string_lossy(),
                    "--name",
                    &env_name,
                    "--yes",
                ],
                config.verbose,
            );
            let rc = result.exit_code();
            match result {
                MicromambaResult::CapturedOutput(ref output) if rc != ExitCode::SUCCESS => {
                    error!("Got a non-zero exit code from micromamba, dumping output:");
                    error!("micromamba stdout:");
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                    error!("micromamba stderr:");
                    println!("{}", String::from_utf8_lossy(&output.stderr));
                }
                MicromambaResult::CapturedOutput(_) if rc == ExitCode::SUCCESS => info!("Done."),
                _ => {}
            }
            result.into()
        }
        Subcommand::List => micromamba(&config, vec!["env", "list"], true).into(),
        Subcommand::Info => micromamba(&config, vec!["info"], true).into(),
        Subcommand::Run(args) => {
            let env_name = env_name(args.common.name, &args.common.env_file)?;
            let mut micromamba_args = vec!["run", "--name", &env_name, &args.command];
            micromamba_args.extend(args.arguments.iter().map(|s| s.as_str()));
            micromamba(&config, micromamba_args, true).into()
        }
        Subcommand::Activate(args) => {
            let Some(shell) = SupportedShell::from_csm_hook() else {
                error!("Your shell does not appear to have the csm hook enabled");
                error!("See 'csm init' for information on how to set up the hook");
                return Err(ExitCode::FAILURE);
            };
            let env_name = env_name(args.name, &args.env_file)?;
            info!("Activating environment '{}'...", env_name);

            // NOTE: Anything to stdout here is *evaluated by the user's shell*
            // Use the logging macros instead for user-facing output!

            // Start by adding the mamba prefix bin to PATH
            let Some(bin_path) = micromamba::bin_path_for_env(&config, &env_name) else {
                error!(
                    "Could not determine binary path for environment '{}'",
                    env_name
                );
                return Err(ExitCode::FAILURE);
            };
            println!("{}", shell.prepend_path(&bin_path));

            // And a few conda-specific vars
            let Some(env_path) = micromamba::path_for_env(&config, &env_name) else {
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
        Subcommand::Deactivate => {
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
        Subcommand::Pack(args) => {
            let env_name = env_name(args.common.name, &args.common.env_file)?;
            let Some(bin_path) = micromamba::bin_path_for_env(&config, &env_name) else {
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
            let Some(env_path) = micromamba::path_for_env(&config, &env_name) else {
                error!("Could not determine path for environment '{}'", env_name);
                return Err(ExitCode::FAILURE);
            };
            let output = args.output.unwrap_or(format!("{}.tar.gz", env_name));
            micromamba(
                &config,
                vec![
                    "run",
                    "--name",
                    &env_name,
                    &binary_name,
                    "--prefix",
                    &env_path.to_string_lossy(),
                    "--output",
                    &output,
                ],
                true,
            )
            .into()
        }
        _ => {
            println!("{:?}", config);
            println!("{:?}", subcommand);
            Ok(())
        }
    }
}
