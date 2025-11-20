use crate::csmrc::Config;

use log::{debug, info};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub enum DownloadError {
    IncompatibleOS,
    BinNotInArchive,
    DownloadDisabled,
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

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::IncompatibleOS => write!(f, "Incompatible OS for micromamba download"),
            DownloadError::BinNotInArchive => {
                write!(f, "micromamba binary not found in downloaded archive")
            }
            DownloadError::DownloadDisabled => write!(
                f,
                "Wanted to download micromamba, but doing so is disabled by csm configuration"
            ),
            DownloadError::IO(e) => write!(f, "IO error: {}", e),
            DownloadError::Reqwest(e) => write!(f, "Failed to download micromamba: {}", e),
        }
    }
}

/// OS-target-specific: Return the final executable name
#[inline]
const fn micromamba_executable_name() -> Result<&'static str, DownloadError> {
    if cfg!(target_os = "linux") {
        Ok("micromamba")
    } else if cfg!(target_os = "windows") {
        Ok("micromamba.exe")
    } else {
        Err(DownloadError::IncompatibleOS)
    }
}

/// Attempt to create the cache directory if necessary, then return it.
fn csm_cache_dir(config: &Config) -> io::Result<PathBuf> {
    let cache = match &config.cache_dir {
        Some(cache_dir) => PathBuf::from(cache_dir),
        _ => match dirs::cache_dir().map(|p| p.join("csm")) {
            Some(cache) => cache,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "could not determine user cache directory",
                ));
            }
        },
    };
    fs::create_dir_all(&cache)?;
    Ok(cache)
}

/// Attempt to download micromamba and store it in the user's cache directory.
///
/// If the file already exists in the cache directory, return the location to
/// it. Otherwise, download it first and then return the location to it.
pub fn download_micromamba(config: &Config) -> Result<PathBuf, DownloadError> {
    let micromamba_exe = micromamba_executable_name()?;
    let micromamba_path = csm_cache_dir(config)?.join(micromamba_exe);

    if micromamba_path.exists() {
        return Ok(micromamba_path);
    }

    // TODO: Do we need to worry about other architectures (aarch64) in addition to OS?
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "win"
    } else {
        return Err(DownloadError::IncompatibleOS);
    };

    let url = format!("https://micro.mamba.pm/api/micromamba/{}-64/latest", os);
    debug!("Going to download {}", url);
    info!("micromamba was not found on path; downloading it now");

    if !config.download_micromamba {
        return Err(DownloadError::DownloadDisabled);
    }

    let response_tarbz2 = reqwest::blocking::get(url)?;
    debug!("Download completed, sending it to BzDecoder");
    let bz2_decoder = bzip2::read::BzDecoder::new(response_tarbz2);
    let mut tar_archive = tar::Archive::new(bz2_decoder);

    debug!("Looking for bin/micromamba in the tarfile");
    let archive_binary_path = if cfg!(target_os = "linux") {
        Path::new("bin").join("micromamba")
    } else if cfg!(target_os = "windows") {
        Path::new("Library").join("bin").join("micromamba.exe")
    } else {
        return Err(DownloadError::IncompatibleOS);
    };

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
