pub mod activate;
pub mod create;
pub mod pack;
pub mod parsing;
pub mod run;
pub mod unpack;

use crate::csmrc::Config;
use crate::env::parsing::env_file::RobotmkEnv;
use crate::micromamba::{self, MicromambaResult, micromamba};
use crate::shell::SupportedShell;

use log::{debug, error, info};
use std::fs;
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
    match subcommand {
        Subcommand::Create(args) => create::run(config, args),
        Subcommand::List => micromamba(&config, vec!["env", "list"], true).into(),
        Subcommand::Info => micromamba(&config, vec!["info"], true).into(),
        Subcommand::Run(args) => run::run(config, args),
        Subcommand::Activate(args) => activate::run(config, args),
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
        Subcommand::Unpack(args) => {
            fn archive_name_to_env_name(archive_path: &Path) -> Option<String> {
                let filename = archive_path.file_name()?.to_str()?;
                filename
                    .strip_suffix(".tar.gz")
                    .or_else(|| filename.strip_suffix(".tgz"))
                    .map(String::from)
            }
            let env_name = match args.common.name {
                Some(name) => name,
                None => match archive_name_to_env_name(&args.archive_path) {
                    Some(name) => {
                        info!(
                            "Using '{}' as environment name, based on archive filename",
                            name
                        );
                        name
                    }
                    None => {
                        error!(
                            "Could not determine environment name from archive filename. Please specify an environment name with --name."
                        );
                        return Err(ExitCode::FAILURE);
                    }
                },
            };

            // TODO: We really need a more generic error type to avoid this kind of mapping everywhere
            let target_env_path = micromamba::create_env_dir(&config, &env_name).map_err(|e| {
                error!("{}", e);
                ExitCode::FAILURE
            })?;

            // Send the archive to flate2 to decompress and untar
            info!(
                "Unpacking archive '{}' to create environment '{}'",
                args.archive_path.display(),
                env_name
            );
            debug!("Opening '{}' for read", args.archive_path.display());
            let archive_file = fs::File::open(&args.archive_path).map_err(|e| {
                error!("{}", e);
                ExitCode::FAILURE
            })?;
            let decompressor = flate2::read::GzDecoder::new(archive_file);
            let mut archive = tar::Archive::new(decompressor);
            archive.unpack(&target_env_path).map_err(|e| {
                error!(
                    "Could not unpack archive to '{}': {}",
                    target_env_path.display(),
                    e
                );
                ExitCode::FAILURE
            })?;
            info!(
                "Successfully unpacked environment to '{}'",
                target_env_path.display()
            );

            info!("Running 'conda-unpack' in the new environment to fix paths...");
            let result = micromamba(
                &config,
                vec!["run", "--name", &env_name, "conda-unpack"],
                config.verbose,
            );
            let rc = result.exit_code();
            dump_micromamba_captured_output_on_error(&result, rc);
            result.into()
        }
    }
}
