use crate::csmrc::Config;
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

    /// Verify SSL/TLS certificates when communicating with external services.
    /// Can be "true" to use the system chain, "false" to disable verification,
    /// or a path to a custom chain (PEM file).
    #[arg(long = "ssl-verify")]
    ssl_verify: Option<String>,

    /// Disable SSL/TLS revokation checking in micromamba
    #[arg(long = "ssl-no-revoke")]
    ssl_no_revoke: bool,
}

pub fn run(mut micromamba: Micromamba, args: Args, config: &Config) -> Result<(), ExitCode> {
    if let Some(path) = &args.setup_file
        && !path.exists()
    {
        error!("Explicit --setup-file was given, but the path does not exist.");
        return Err(ExitCode::FAILURE);
    }

    let (ssl_verify, ssl_bundle) = match &args.ssl_verify {
        Some(b) if b == "false" => (false, None),
        Some(b) if b == "true" => (true, None),
        Some(bundle) => (true, Some(bundle.clone())),
        None => (
            config.env_create.ssl_verify,
            config.env_create.ssl_bundle.clone(),
        ),
    };

    if ssl_verify && let Some(ssl_bundle) = ssl_bundle {
        micromamba.set_env_var("PIP_CERT", &ssl_bundle);
        micromamba.set_env_var("REQUESTS_CA_BUNDLE", &ssl_bundle);
        micromamba.set_env_var("CURL_CA_BUNDLE", &ssl_bundle);
    } else if !ssl_verify {
        micromamba.set_env_var("PIP_CERT", "");
        micromamba.set_env_var("REQUESTS_CA_BUNDLE", "");
        micromamba.set_env_var("CURL_CA_BUNDLE", "");
        micromamba.set_env_var(
            "PIP_TRUSTED_HOST",
            "pypi.org files.pythonhosted.org pypi.pythonhosted.org",
        );
        micromamba.set_env_var("PIP_INDEX_URL", "https://pypi.org/simple");
    }

    let ssl_no_revoke = args.ssl_no_revoke || config.env_create.ssl_no_revoke;
    if ssl_no_revoke {
        micromamba.set_env_var("MAMBA_SSL_NO_REVOKE", "true");
    }

    let env_name = env_name(args.common.name, &args.common.env_file)?;
    info!(
        "Creating environment '{}' - this may take some time...",
        env_name
    );

    let env_file = args.common.env_file.to_string_lossy();
    let mut mm_args = vec![
        "env", "create", "--file", &env_file, "--name", &env_name, "--yes",
    ];
    if !ssl_verify {
        mm_args.push("--ssl-verify=<false>");
    }
    let result = micromamba.stream_if_verbose(mm_args);
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
