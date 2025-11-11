use crate::csmrc::Config;
use crate::micromamba::{self, MicromambaResult, micromamba};
use crate::shell::SupportedShell;

use log::{debug, error, info};
use serde::Deserialize;
use std::io::{Error, ErrorKind};
use std::path::Component;
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
    /// ???
    Pack,
    /// ???
    Unpack,
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
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// If specified, the name of the environment. If not specified, csm will
    /// look to robotmk-env.yaml for a "name" field to use instead. As a last
    /// resort, the current directory name will be used
    #[arg(short, long, value_name = "ENV_NAME")]
    name: Option<String>,
    /// The command to run
    #[arg(value_name = "COMMAND")]
    command: String,
    /// Arguments to pass to the command
    #[arg(value_name = "ARGS")]
    arguments: Vec<String>,
}

/// Contains the fields we need from a parsed `robotmk-env.yml` file.
#[derive(Deserialize)]
struct RobotmkEnv {
    /// The name of the environment
    name: Option<String>,
}

/// Attempt to parse a robotmk-env.yaml in the current directory.
fn parse_robotmk_env_yaml() -> Result<RobotmkEnv, std::io::Error> {
    // TODO: Should we handle .yml too?
    let contents = std::fs::read_to_string("robotmk-env.yaml")?;
    serde_yaml_ng::from_str(&contents).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

pub fn determine_env_name(explicit_name: Option<String>) -> Option<String> {
    // If someone gave an explicit --name, use that first.
    if let Some(name) = explicit_name {
        debug!("Using '{}' as env name, given by CLI argument", name);
        return Some(name);
    }

    // Fallback 1: Look for a name key in robotmk-env.yaml
    // We ignore errors from parse_robotmk_env_yaml() here, we'll fall back
    // below if we can't parse it for some reason
    if let Ok(env) = parse_robotmk_env_yaml()
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

pub fn run(config: Config, subcommand: Subcommand) -> ExitCode {
    match subcommand {
        Subcommand::Create(args) => {
            let Some(env_name) = determine_env_name(args.name) else {
                error!("No environment name could be determined. You can specify one with --name");
                return ExitCode::FAILURE;
            };
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
                    "robotmk-env.yaml",
                    "--name",
                    &env_name,
                    "--yes",
                ],
                config.verbose,
            );
            let rc = result.exit_code();
            match result {
                MicromambaResult::CapturedOutput(output) if rc != ExitCode::SUCCESS => {
                    error!("Got a non-zero exit code from micromamba, dumping output:");
                    error!("micromamba stdout:");
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                    error!("micromamba stderr:");
                    println!("{}", String::from_utf8_lossy(&output.stderr));
                }
                MicromambaResult::CapturedOutput(_) if rc == ExitCode::SUCCESS => info!("Done."),
                _ => {}
            }
            rc
        }
        Subcommand::List => micromamba(&config, vec!["env", "list"], true).exit_code(),
        Subcommand::Info => micromamba(&config, vec!["info"], true).exit_code(),
        Subcommand::Run(args) => {
            let Some(env_name) = determine_env_name(args.name) else {
                error!("No environment name could be determined. You can specify one with --name");
                return ExitCode::FAILURE;
            };
            let mut micromamba_args = vec!["run", "--name", &env_name, &args.command];
            micromamba_args.extend(args.arguments.iter().map(|s| s.as_str()));
            micromamba(&config, micromamba_args, true).exit_code()
        }
        Subcommand::Activate(args) => {
            let Some(shell) = SupportedShell::from_csm_hook() else {
                error!("Your shell does not appear to have the csm hook enabled");
                error!("See 'csm init' for information on how to set up the hook");
                return ExitCode::FAILURE;
            };

            let Some(env_name) = determine_env_name(args.name) else {
                error!("No environment name could be determined. You can specify one with --name");
                return ExitCode::FAILURE;
            };

            info!("Activating environment '{}'...", env_name);

            // NOTE: Anything to stdout here is *evaluated by the user's shell*
            // Use the logging macros instead for user-facing output!

            // Start by adding the mamba prefix bin to PATH
            let Some(mut env_path) = micromamba::path_for_env(&config, &env_name) else {
                error!("Could not determine path for environment '{}'", env_name);
                return ExitCode::FAILURE;
            };
            let bin = if cfg!(windows) { "Scripts" } else { "bin" };
            env_path.push(bin);
            println!("{}", shell.prepend_path(&env_path));

            // And a few conda-specific vars
            println!("{}", shell.set_env_var("CONDA_DEFAULT_ENV", &env_name));
            println!(
                "{}",
                shell.set_env_var("CONDA_PREFIX", &env_path.to_string_lossy())
            );
            println!("{}", shell.set_env_var("CONDA_SHLVL", "1"));

            ExitCode::SUCCESS
        }
        Subcommand::Deactivate => {
            let Some(shell) = SupportedShell::from_csm_hook() else {
                error!("Your shell does not appear to have the csm hook enabled");
                error!("See 'csm init' for information on how to set up the hook");
                return ExitCode::FAILURE;
            };
            println!("{}", shell.restore_and_unset_env_var("PATH"));
            println!("{}", shell.restore_and_unset_env_var("CONDA_DEFAULT_ENV"));
            println!("{}", shell.restore_and_unset_env_var("CONDA_PREFIX"));
            println!("{}", shell.restore_and_unset_env_var("CONDA_SHLVL"));
            ExitCode::SUCCESS
        }
        _ => {
            println!("{:?}", config);
            println!("{:?}", subcommand);
            ExitCode::SUCCESS
        }
    }
}
