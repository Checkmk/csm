pub mod activate;
pub mod create;
pub mod deactivate;
pub mod pack;
pub mod parsing;
pub mod run;
pub mod unpack;

use crate::csmrc::Config;
use crate::env::parsing::env_file::RobotmkEnv;
use crate::micromamba::Micromamba;
use crate::micromamba::result::MicromambaResult;

use log::{debug, error, info};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Create an environment
    Create(create::Args),
    /// Activate an environment
    Activate(activate::Args),
    /// Deactivate an environment
    Deactivate,
    /// Run an executable in an environment
    Run(run::Args),
    /// Create an archive from an environment. Requires `conda-pack` to be in the environment.
    Pack(pack::Args),
    /// Unpack an archive to create an environment from it.
    Unpack(unpack::Args),
    /// List existing environments
    List,
    /// Display information about the micromamba setup
    Info,
}

#[derive(Debug, clap::Args)]
pub struct CommonArgs {
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

pub fn determine_env_name(explicit_name: Option<String>, env_yaml_path: &Path) -> Option<String> {
    // If someone gave an explicit --name, use that first.
    if let Some(name) = explicit_name {
        debug!("Using '{}' as env name, given by CLI argument", name);
        return Some(name);
    }

    // Fallback 1: Look for a name key in robotmk-env.yaml
    // We ignore errors from from_path() here, we'll fall back
    // below if we can't parse it for some reason
    if let Ok(env) = RobotmkEnv::from_path(env_yaml_path)
        && let Some(name) = env.name
    {
        debug!("Using '{}' as env name, found in {:?}", name, env_yaml_path);
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

pub fn env_name(explicit_name: Option<String>, env_yaml_path: &Path) -> Result<String, ExitCode> {
    match determine_env_name(explicit_name, env_yaml_path) {
        Some(name) => Ok(name),
        None => {
            error!("No environment name could be determined. You can specify one with --name");
            Err(ExitCode::FAILURE)
        }
    }
}

fn dump_micromamba_captured_output_on_error(result: &MicromambaResult, rc: ExitCode) {
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
}

pub fn run(config: Config, subcommand: Subcommand) -> Result<(), ExitCode> {
    let micromamba = Micromamba::new(&config);

    match subcommand {
        Subcommand::Create(args) => create::run(config, args),
        Subcommand::List => micromamba.stream(vec!["env", "list"]).into(),
        Subcommand::Info => micromamba.stream(vec!["info"]).into(),
        Subcommand::Run(args) => run::run(config, args),
        Subcommand::Activate(args) => activate::run(config, args),
        Subcommand::Deactivate => deactivate::run(),
        Subcommand::Pack(args) => pack::run(config, args),
        Subcommand::Unpack(args) => unpack::run(config, args),
    }
}
