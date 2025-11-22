use crate::env::dump_micromamba_captured_output_on_error;
use crate::micromamba::Micromamba;

use log::{debug, error, info};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// The name of the environment to create. By default, will be determined
    /// from the given archive filename: <env_name>.tar.gz
    #[arg(short, long, value_name = "ENV_NAME")]
    pub name: Option<String>,
    /// Path to a packed environment archive (ending in .tar.gz)
    #[arg(value_name = "ARCHIVE")]
    pub archive_path: PathBuf,
}

fn archive_name_to_env_name(archive_path: &Path) -> Option<String> {
    let filename = archive_path.file_name()?.to_str()?;
    filename
        .strip_suffix(".tar.gz")
        .or_else(|| filename.strip_suffix(".tgz"))
        .map(String::from)
}

pub fn run(micromamba: Micromamba, args: Args) -> Result<(), ExitCode> {
    let env_name = match args.name {
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
    let target_env_path = micromamba.create_env_dir(&env_name).map_err(|e| {
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
    let result = micromamba.stream_if_verbose(vec!["run", "--name", &env_name, "conda-unpack"]);
    let rc = result.exit_code();
    dump_micromamba_captured_output_on_error(&result, rc);
    result.into()
}
