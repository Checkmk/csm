//! This module deals with `micromamba` - obtaining it, calling it, etc.

use crate::csmrc::Config;
use log::{debug, error, info};
use std::collections::HashMap;
use std::process::{Command, ExitCode, ExitStatus};

/// The result from trying to shell out to `micromamba`.
///
/// It would be better if we could "accumulate" errors as we try different
/// fallbacks to run `micromamba`, something like Result/Either, but with an
/// accumulating Applicative on the error side, akin to the "validation" package
/// in Haskell. Alas, this does not seem to exist in Rust, so we drop the errors
/// as we try to determine a working `micromamba` and just report whether or not
/// we were able to do so at the end. (Of course, we log along the way in
/// `micromamba()`.)
pub enum MicromambaResult {
    /// We were run in no-op mode, so we didn't actually call out to it
    Noop,
    /// We were able to successfully call it and get a result
    Ok(ExitStatus),
    /// We were unable to find or create a working `micromamba`
    CouldNotRun,
}

impl MicromambaResult {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Ok(exit_status) => exit_status
                .code()
                .map(|c| ExitCode::from(c as u8))
                .unwrap_or(ExitCode::FAILURE),
            Self::Noop => ExitCode::SUCCESS,
            Self::CouldNotRun => ExitCode::FAILURE,
        }
    }
}

/// Return a [`Command`] ready to shell out to `micromamba` with the appropriate
/// environment variables set based on configuration.
pub fn micromamba_at(path: &str, config: &Config, args: Vec<&str>) -> Command {
    let mut env_vars: HashMap<&str, String> = HashMap::new();

    if let Some(mamba_root_prefix) = &config.mamba_root_prefix {
        env_vars.insert("MAMBA_ROOT_PREFIX", mamba_root_prefix.to_string());
    }

    let mut cmd = Command::new(path);
    cmd.args(args);
    cmd.envs(env_vars);
    if config.noop_mode {
        info!("Would run: {:?}", cmd);
    } else {
        debug!("About to run: {:?}", cmd);
    }
    cmd
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
pub fn micromamba(config: &Config, args: Vec<&str>) -> MicromambaResult {
    let mut cmd = micromamba_at("micromamba", config, args);

    if config.noop_mode {
        // Do nothing. micromamba_at() already logged what we're about to run.
        return MicromambaResult::Noop;
    }

    // First we try from $PATH
    if let Ok(mut child) = cmd.spawn() {
        debug!("Used micromamba from $PATH");
        match child.wait() {
            Ok(exit_status) => return MicromambaResult::Ok(exit_status),
            Err(e) => {
                // In this case don't try to download one, there is probably a
                // bigger issue.
                error!("We found a micromamba binary, but failed to wait for it to run");
                error!("Error was: {}", e);
                return MicromambaResult::CouldNotRun;
            }
        }
    }

    // If we weren't successful there, we download micromamba to the user cache
    // directory.

    // TODO

    // Finally, if we couldn't run the downloaded one either, just bail out
    error!("Could not find a suitable micromamba binary to run");
    error!(
        "Please install micromamba manually, ensure it is executable, and place it somewhere in $PATH"
    );
    MicromambaResult::CouldNotRun
}
