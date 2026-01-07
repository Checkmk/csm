use csm::CSMResult;
use csm::csmrc::Config;
use csm::env;
use csm::init;
use csm::robot;
use csm::shell;

use clap::{CommandFactory, Parser, Subcommand};
use log::{LevelFilter, debug, error, info, warn};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
#[cfg(windows)]
use windows_registry::LOCAL_MACHINE;

#[derive(Parser, Debug)]
#[command(version)]
/// Checkmk Synthetic Monitoring command-line tool
struct Cli {
    /// Enable verbose debugging output
    #[arg(short, long)]
    verbose: bool,

    /// Don't make any changes, only print what would happen
    #[arg(short = 'n', long = "noop")]
    noop_mode: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage Robot environments
    #[command(subcommand)]
    Env(env::Subcommand),

    /// Manage Robotmk robots
    #[command(subcommand)]
    Robot(robot::Subcommand),

    /// Initialize shell environment for csm
    Init {
        shell: Option<shell::SupportedShell>,
        /// Generate the actual shell code to be evaluated
        #[arg(long)]
        code: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Set up logging
    let default_verbosity = if cli.verbose {
        LevelFilter::Debug
    } else {
        // We use info level for no-op mode messages.
        LevelFilter::Info
    };
    let mut env_logger_builder = env_logger::Builder::new();
    env_logger_builder.filter_level(default_verbosity);
    env_logger_builder.parse_default_env();
    env_logger_builder.format_timestamp(None);
    env_logger_builder.init();

    let mut config = match Config::from_csmrc() {
        Ok(config) => config,
        Err(err) => {
            error!("Failed to parse .csmrc: {}", err);
            return ExitCode::FAILURE;
        }
    };

    if cli.noop_mode {
        config.noop_mode = true;
    }

    if cli.verbose {
        config.verbose = true;
    }

    let Some(home) = std::env::home_dir() else {
        error!("Failed to determine home directory");
        return ExitCode::FAILURE;
    };

    let mambarc = include_str!("../templates/mambarc");
    let csmrc = include_str!("../templates/csmrc");
    for (dotfile, contents) in [(".mambarc", mambarc), (".csmrc", csmrc)] {
        let path = home.join(dotfile);
        if let Err(e) = create_config_file(&config, &path, contents) {
            warn!("Could not create {}, but continuing: {}", path.display(), e);
        }
    }

    #[cfg(windows)]
    if !config.skip_longpaths_check
        && let Err(e) = windows_set_longpaths()
    {
        warn!(
            "Error while checking or enabling LongPaths for Windows: {}",
            e
        );
        eprintln!(
            "Continuing without LongPaths support can have unwanted effects and is not recommended."
        );
        eprint!("Continue anyway? [y/N]: ");
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            error!("Failed to get user input");
            return ExitCode::FAILURE;
        }
        match input.trim() {
            "y" | "Y" | "yes" => info!(
                "You can add the line 'skip_longpaths_check: true' to {} to skip this prompt in the future",
                home.join(".csmrc").display()
            ),
            _ => {
                error!("Exiting.");
                return ExitCode::FAILURE;
            }
        }
    }

    match cli.command {
        Command::Env(sub) => env::run(config, sub).finish(),
        Command::Robot(sub) => robot::run(config, sub).finish(),
        Command::Init { shell, code } => {
            let mut cmd = Cli::command();
            init::run(config, shell, code, &mut cmd).finish()
        }
    }
}

/// Create a configuration file (e.g. .mambarc or .csmrc) if it does not exist.
fn create_config_file(config: &Config, path: &Path, contents: &str) -> std::io::Result<()> {
    if config.noop_mode && !path.exists() {
        info!("Would create {}", path.display());
        return Ok(());
    }

    match File::create_new(path) {
        Ok(mut file) => {
            file.write_all(contents.trim_start().as_bytes())?;
            debug!("Created {}", path.display());
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            debug!("File {} already exists, not creating", path.display())
        }
        Err(e) => return Err(e),
    }
    Ok(())
}

/// On windows, check if LongPaths are enabled in the registry, and enable them
/// if they are not.
///
/// The *check* is done with read-only access to the registry path and should
/// not require administrator privileges. The *modification* likely requires
/// administrative privileges and we re-open the key with write access for it.
#[cfg(windows)]
fn windows_set_longpaths() -> windows_registry::Result<()> {
    let reg_path = r"SYSTEM\CurrentControlSet\Control\FileSystem";
    let ro_key = LOCAL_MACHINE.open(reg_path)?;
    match ro_key.get_u32("LongPathsEnabled") {
        Ok(1) => {
            debug!("Windows LongPathsEnabled setting is already enabled");
            Ok(())
        }
        Ok(_) => {
            info!(
                "Enabling LongPaths in the registry, this may require administrative permissions"
            );
            let rw_key = LOCAL_MACHINE.create(reg_path)?;
            rw_key.set_u32("LongPathsEnabled", 1)
        }
        e @ Err(_) => e.map(|_| ()),
    }
}
