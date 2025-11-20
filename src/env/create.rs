use crate::env::parsing::setup_file::RobotmkSetup;
use crate::env::{CommonArgs, dump_micromamba_captured_output_on_error, env_name};
use crate::micromamba::Micromamba;

use log::{debug, error, info, warn};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub common: CommonArgs,

    /// If specified, overrides the post-creation setup file [default: robotmk-setup.yaml]
    #[arg(long = "setup-file", value_name = "PATH")]
    pub setup_file: Option<PathBuf>,
}

pub fn run(micromamba: Micromamba, args: Args) -> Result<(), ExitCode> {
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
    let result = micromamba.stream_if_verbose(vec![
        "env",
        "create",
        "--file",
        &args.common.env_file.to_string_lossy(),
        "--name",
        &env_name,
        "--yes",
    ]);
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
            Some(setup) => setup.run_post_create(&micromamba, &env_name)?,
            None => debug!("No usable setup file, skipping post_create"),
        }
    }
    result.into()
}
