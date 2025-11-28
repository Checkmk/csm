//! This module deals with `micromamba` - obtaining it, calling it, etc.

mod download;
pub mod result;

use crate::csmrc::Config;
use crate::micromamba::download::download_micromamba;
use crate::micromamba::result::MicromambaResult;

use log::{debug, error, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize)]
pub struct MicromambaInfo {
    #[serde(rename(deserialize = "env location"))]
    pub env_location: String,
}

pub struct Micromamba<'a> {
    config: &'a Config,
}

fn block_on_child_exit(child: &mut std::process::Child) -> MicromambaResult {
    match child.wait() {
        Ok(exit_status) => {
            debug!("micromamba exited with status: {}", exit_status);
            MicromambaResult::StreamedOutput(exit_status)
        }
        Err(e) => {
            error!("We found a micromamba binary, but failed to wait for it to run");
            error!("Error was: {}", e);
            MicromambaResult::CouldNotRun
        }
    }
}

fn exec_micromamba(cmd: &mut Command, stream_output: bool) -> MicromambaResult {
    let result = if stream_output {
        match cmd.spawn() {
            Ok(mut child) => block_on_child_exit(&mut child),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                debug!("Could not run micromamba at specified path: {}", e);
                MicromambaResult::NotFound
            }
            Err(e) => {
                error!("We found a micromamba binary, but failed to run it");
                error!("Error was: {}", e);
                MicromambaResult::CouldNotRun
            }
        }
    } else {
        match cmd.output() {
            Ok(output) => MicromambaResult::CapturedOutput(output),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                debug!("Could not run micromamba at specified path: {}", e);
                MicromambaResult::NotFound
            }
            Err(e) => {
                error!("We found a micromamba binary, but failed to run it");
                error!("Error was: {}", e);
                MicromambaResult::CouldNotRun
            }
        }
    };

    if cfg!(windows)
        && result
            .exit_status()
            .and_then(|s| s.code())
            .is_some_and(|s| s == 0xc0000135u32 as i32)
    {
        error!("Windows reported a missing DLL required to run micromamba");
        error!("This error can very likely be solved by installing the VC++ runtime libraries");
        error!(
            "See https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170#latest-supported-redistributable-version for more information"
        );
    }

    result
}

impl<'a> Micromamba<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Return a [`Command`] ready to shell out to `micromamba` with the appropriate
    /// environment variables set based on configuration.
    fn micromamba_at(&self, path: &str, args: &Vec<&str>) -> Command {
        let mut env_vars: HashMap<&str, String> = HashMap::new();

        if let Some(mamba_root_prefix) = &self.config.mamba_root_prefix {
            env_vars.insert("MAMBA_ROOT_PREFIX", mamba_root_prefix.to_string());
        }

        let mut cmd = Command::new(path);
        cmd.args(args);
        cmd.envs(env_vars);
        if self.config.noop_mode {
            info!("Would run: {:?}", cmd);
        } else {
            debug!("About to run: {:?}", cmd);
        }
        cmd
    }

    /// Execute micromamba and stream the output to the console.
    ///
    /// Standard input is inherited from the calling console.
    pub fn stream(&self, args: Vec<&str>) -> MicromambaResult {
        self.micromamba(args, true)
    }

    /// Execute micromamba and capture the output, to be displayed in the event
    /// of an error.
    ///
    /// Standard input is dropped.
    pub fn capture(&self, args: Vec<&str>) -> MicromambaResult {
        self.micromamba(args, false)
    }

    /// Execute micromamba and stream the output if we are running in verbose mode.
    /// Otherwise, capture the output to be displayed in the event of an error.
    pub fn stream_if_verbose(&self, args: Vec<&str>) -> MicromambaResult {
        if self.config.verbose {
            self.micromamba(args, true)
        } else {
            self.micromamba(args, false)
        }
    }

    /// Run `micromamba` and return the result, if able.
    ///
    /// We need a `micromamba` binary to work with. If one is not present, attempt
    /// to download and install `micromamba` into the user's cache directory.
    ///
    /// 1. If there is already a `micromamba` command in $PATH, we use it.
    /// 2. Otherwise, download micromamba and install it somewhere in the user
    ///    cache directory. (We cannot rely on this - it could be that the user's
    ///    cache directory is mounted noexec or similar, but we try.)
    ///
    /// Alternative approaches that we do not take here currently:
    /// - On Linux, we *could* in theory use memfd_create + fexecve to embed the app
    ///   and run it from memory. This won't work on Windows.
    ///
    /// - We *could* embed the micromamba binary in our binary (Windows or Linux
    ///   based on compile target) and write it to the user cache directory rather
    ///   than downloading it. But this inflates our binary size.
    fn micromamba(&self, args: Vec<&str>, stream_output: bool) -> MicromambaResult {
        let mut cmd = self.micromamba_at("micromamba", &args);

        if self.config.noop_mode {
            // Do nothing. micromamba_at() already logged what we're about to run.
            return MicromambaResult::Noop;
        }

        // If we were able to get a result using micromamba found in $PATH, then
        // we're done.
        match exec_micromamba(&mut cmd, stream_output) {
            ok @ (MicromambaResult::StreamedOutput(_) | MicromambaResult::CapturedOutput(_)) => {
                debug!("Ran micromamba found in $PATH");
                return ok;
            }
            MicromambaResult::CouldNotRun => {
                // In this case, bail out and let the user fix their micromamba
                // installation.
                debug!("micromamba found in $PATH could not be run, aborting");
                return MicromambaResult::CouldNotRun;
            }
            _ => {}
        }

        // If we weren't successful there, we download micromamba to the user cache
        // directory.
        debug!("micromamba not found in $PATH, falling back to cache");
        let downloaded_path = match download_micromamba(self.config) {
            Ok(path) => path,
            Err(e) => {
                error!("Could not download micromamba: {}", e);
                return MicromambaResult::CouldNotRun;
            }
        };
        let mut cmd = self.micromamba_at(&downloaded_path.to_string_lossy(), &args);
        match exec_micromamba(&mut cmd, stream_output) {
            ok @ (MicromambaResult::StreamedOutput(_) | MicromambaResult::CapturedOutput(_)) => {
                debug!(
                    "Ran downloaded/cached micromamba at {}",
                    downloaded_path.display()
                );
                return ok;
            }
            MicromambaResult::CouldNotRun => {
                debug!(
                    "Downloaded micromamba at {} could not be run",
                    downloaded_path.display()
                );
            }
            _ => {}
        }

        // Finally, if we couldn't run the downloaded one either, just bail out
        error!("Could not find a suitable micromamba binary to run");
        error!(
            "Please install micromamba manually, ensure it is executable, and place it somewhere in $PATH"
        );
        MicromambaResult::CouldNotRun
    }

    /// Query micromamba to try to determine the path for an environment
    pub fn path_for_env(&self, name: &str) -> Option<PathBuf> {
        if self.config.noop_mode {
            return Some(PathBuf::from("/no-op/mode/path/for/env"));
        }
        let result = self.capture(vec!["info", "--name", name, "--json"]);
        let MicromambaResult::CapturedOutput(output) = result else {
            return None;
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let info: MicromambaInfo = serde_json::from_str(&stdout).ok()?;
        Some(info.env_location.into())
    }

    /// Return an OS-specific path to where micromamba stores (most?) binaries.
    pub fn bin_path_for_env(&self, name: &str) -> Option<PathBuf> {
        let os_specific_bin = if cfg!(windows) { "Scripts" } else { "bin" };
        self.path_for_env(name).map(|p| p.join(os_specific_bin))
    }

    /// Create a directory for a new environment, if it does not already exist.
    ///
    /// If `force` is true, delete any pre-existing environment/directory at this path.
    pub fn create_env_dir(&self, name: &str, force: bool) -> Result<PathBuf, std::io::Error> {
        if self.config.noop_mode {
            info!("Using fake directory path due to no-op mode");
            return Ok(PathBuf::from("/no-op/mode/path/for/env"));
        }
        let env_path = match self.path_for_env(name) {
            Some(path) => path,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Could not determine path for environment '{}'", name),
                ));
            }
        };
        if env_path.exists() {
            if force {
                debug!(
                    "Removing pre-existing directory at '{}'",
                    env_path.display()
                );
                std::fs::remove_dir_all(&env_path)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!(
                        "The target environment directory '{}' already exists",
                        env_path.display()
                    ),
                ));
            }
        }
        std::fs::create_dir_all(&env_path)?;
        Ok(env_path)
    }
}
