pub mod activate;
pub mod create;
pub mod pack;
pub mod run;
pub mod unpack;

use crate::csmrc::Config;
use crate::micromamba::{self, MicromambaResult, micromamba};
use crate::shell::SupportedShell;

use log::{debug, error, info, warn};
use serde::Deserialize;
use std::fs;
use std::io::{Error, ErrorKind};
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

/// Contains the fields we need from a parsed environment file.
#[derive(Deserialize)]
struct RobotmkEnv {
    /// The name of the environment
    name: Option<String>,
}

/// Contains the fields we need from a parsed setup file.
#[derive(Deserialize)]
struct RobotmkSetup {
    /// Commands to run in the environment after it has been created
    post_build_commands: Option<Vec<PostBuildCommand>>,
}

#[derive(Deserialize)]
struct PostBuildCommand {
    name: Option<String>,
    command: Vec<String>,
}

impl RobotmkSetup {
    /// Attempt to parse a setup file.
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_yaml_ng::from_str(&contents).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }

    fn run_post_create(self, config: &Config, env_name: &str) -> Result<(), ExitCode> {
        for command in self.post_build_commands.unwrap_or_default() {
            info!(
                "Start post-create: {}",
                command.name.unwrap_or(format!("{:?}", command.command))
            );
            let mut args = vec!["run", "--name", &env_name];
            args.extend(command.command.iter().map(|s| s.as_str()));
            let result = micromamba(config, args, config.verbose);
            let rc = result.exit_code();
            dump_micromamba_captured_output_on_error(&result, rc);
            if rc != ExitCode::SUCCESS {
                error!("The command returned exited with an error code");
                error!("Not executing further post-create commands");
                return Err(rc);
            }
        }
        Ok(())
    }
}

/// Attempt to parse an environment file.
fn parse_env_yaml<P: AsRef<Path>>(path: P) -> Result<RobotmkEnv, std::io::Error> {
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

fn env_name(explicit_name: Option<String>, env_yaml_path: &Path) -> Result<String, ExitCode> {
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
        Subcommand::Create(args) => {
            if let Some(path) = &args.setup_file
                && !path.exists()
            {
                error!("Explicit --setup-file was given, but the path does not exist.");
                return Err(ExitCode::FAILURE);
            }

            let env_name = env_name(args.common.name, &args.common.env_file)?;
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
                    &args.common.env_file.to_string_lossy(),
                    "--name",
                    &env_name,
                    "--yes",
                ],
                config.verbose,
            );
            let rc = result.exit_code();
            dump_micromamba_captured_output_on_error(&result, rc);
            if rc == ExitCode::SUCCESS {
                let (filename, required) = match args.setup_file {
                    None => ("robotmk-setup.yaml".into(), false),
                    Some(path) => (path, true),
                };
                let setup = match RobotmkSetup::from_path(&filename) {
                    Err(e) if e.kind() == ErrorKind::NotFound => {
                        if required {
                            // Handled above, but theoretical race here, so handle it
                            error!("The specified setup file was not found");
                            None
                        } else {
                            debug!(
                                "No explicit setup file path given, and the default \
                                 robotmk-setup.yaml was not found, continuing."
                            );
                            None
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Found setup file '{}', but could not read it: {}",
                            filename.display(),
                            e
                        );
                        None
                    }
                    Ok(setup) => {
                        debug!("Read setup file '{}'", filename.display());
                        Some(setup)
                    }
                };
                match setup {
                    Some(setup) => setup.run_post_create(&config, &env_name)?,
                    None => debug!("No usable setup file, skipping post_create"),
                }
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
            let env_name = env_name(args.common.name, &args.common.env_file)?;
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
