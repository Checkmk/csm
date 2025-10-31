//! This module deals with `micromamba` - obtaining it, calling it, etc.

use crate::csmrc::Config;
use log::{debug, error, info};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
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
    NotFound,
    /// We found a micromamba binary, but could not run it
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
            _ => ExitCode::FAILURE,
        }
    }
}

enum DownloadError {
    IncompatibleOS,
    BinNotInArchive,
    IO(io::Error),
    Reqwest(reqwest::Error),
}

impl From<io::Error> for DownloadError {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<reqwest::Error> for DownloadError {
    fn from(err: reqwest::Error) -> Self {
        Self::Reqwest(err)
    }
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::IncompatibleOS => write!(f, "Incompatible OS for micromamba download"),
            DownloadError::BinNotInArchive => {
                write!(f, "micromamba binary not found in downloaded archive")
            }
            DownloadError::IO(e) => write!(f, "IO error: {}", e),
            DownloadError::Reqwest(e) => write!(f, "Failed to download micromamba: {}", e),
        }
    }
}

/// Return a [`Command`] ready to shell out to `micromamba` with the appropriate
/// environment variables set based on configuration.
pub fn micromamba_at(path: &str, config: &Config, args: &Vec<&str>) -> Command {
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

fn block_on_child_exit(child: &mut std::process::Child) -> MicromambaResult {
    match child.wait() {
        Ok(exit_status) => {
            debug!("micromamba exited with status: {}", exit_status);
            MicromambaResult::Ok(exit_status)
        }
        Err(e) => {
            error!("We found a micromamba binary, but failed to wait for it to run");
            error!("Error was: {}", e);
            MicromambaResult::CouldNotRun
        }
    }
}

fn exec_micromamba(cmd: &mut Command) -> MicromambaResult {
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
    let mut cmd = micromamba_at("micromamba", config, &args);

    if config.noop_mode {
        // Do nothing. micromamba_at() already logged what we're about to run.
        return MicromambaResult::Noop;
    }

    // If we were able to get a result using micromamba found in $PATH, then
    // we're done.
    match exec_micromamba(&mut cmd) {
        ok @ MicromambaResult::Ok(_) => {
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
    let downloaded_path = match download_micromamba(config) {
        Ok(path) => path,
        Err(e) => {
            error!("Could not download micromamba: {}", e);
            return MicromambaResult::CouldNotRun;
        }
    };
    let mut cmd = micromamba_at(&downloaded_path.to_string_lossy(), config, &args);
    match exec_micromamba(&mut cmd) {
        ok @ MicromambaResult::Ok(_) => {
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

/// Attempt to create the cache directory if necessary, then return it.
fn csm_cache_dir(config: &Config) -> std::io::Result<PathBuf> {
    let cache = if let Some(cache_dir) = &config.cache_dir {
        PathBuf::from(cache_dir)
    } else {
        let Some(cache) = dirs::cache_dir().map(|p| p.join("csm")) else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine user cache directory",
            ));
        };
        cache
    };
    fs::create_dir_all(&cache)?;
    Ok(cache)
}

/// Attempt to download micromamba and store it in the user's cache directory.
///
/// If the file already exists in the cache directory, return the location to
/// it. Otherwise, download it first and then return the location to it.
fn download_micromamba(config: &Config) -> Result<PathBuf, DownloadError> {
    let cache_dir = csm_cache_dir(config)?;
    let micromamba_path = if cfg!(target_os = "linux") {
        cache_dir.join("micromamba")
    } else if cfg!(target_os = "windows") {
        cache_dir.join("micromamba.exe")
    } else {
        return Err(DownloadError::IncompatibleOS);
    };

    if micromamba_path.exists() {
        return Ok(micromamba_path);
    }

    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "win"
    } else {
        return Err(DownloadError::IncompatibleOS);
    };

    let archive_binary_path = if cfg!(target_os = "linux") {
        Path::new("bin").join("micromamba")
    } else if cfg!(target_os = "windows") {
        Path::new("Library").join("bin").join("micromamba.exe")
    } else {
        return Err(DownloadError::IncompatibleOS);
    };

    // TODO: Do we need to worry about other architectures? (aarch64)
    let url = format!("https://micro.mamba.pm/api/micromamba/{}-64/latest", os);
    debug!("Going to download {}", url);
    info!("micromamba was not found on path; downloading it now");
    let response_tarbz2 = reqwest::blocking::get(url)?;
    debug!("Download completed, sending it to BzDecoder");
    let bz2_decoder = bzip2::read::BzDecoder::new(response_tarbz2);
    let mut tar_archive = tar::Archive::new(bz2_decoder);

    debug!("Looking for bin/micromamba in the tarfile");
    for entry in tar_archive.entries()? {
        let mut entry = entry?;
        if let Ok(path) = entry.path()
            && path == archive_binary_path
        {
            debug!(
                "Found it, writing it to disk at {}",
                micromamba_path.display()
            );
            let mut out = fs::File::create(&micromamba_path)?;
            io::copy(&mut entry, &mut out)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = out.metadata()?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&micromamba_path, perms)?;
            }

            return Ok(micromamba_path);
        }
    }

    Err(DownloadError::BinNotInArchive)
}
