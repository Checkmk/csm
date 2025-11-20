use crate::env::dump_micromamba_captured_output_on_error;
use crate::micromamba::Micromamba;

use log::{error, info};
use serde::Deserialize;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::process::ExitCode;

/// Contains the fields we need from a parsed setup file.
#[derive(Deserialize)]
pub struct RobotmkSetup {
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
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_yaml_ng::from_str(&contents).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }

    pub fn run_post_create(self, micromamba: &Micromamba, env_name: &str) -> Result<(), ExitCode> {
        for command in self.post_build_commands.unwrap_or_default() {
            info!(
                "Start post-create: {}",
                command.name.unwrap_or(format!("{:?}", command.command))
            );
            let mut args = vec!["run", "--name", &env_name];
            args.extend(command.command.iter().map(|s| s.as_str()));
            let result = micromamba.stream_if_verbose(args);
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
