use crate::csmrc::Config;

use log::{debug, info};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::Duration;

pub enum DownloadError {
    IncompatibleOS,
    IncompatibleArch,
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
            DownloadError::IncompatibleArch => {
                write!(
                    f,
                    "Incompatible architecture/OS combination for micromamba download"
                )
            }
            DownloadError::BinNotInArchive => {
                write!(f, "micromamba binary not found in downloaded archive")
            }
            DownloadError::DownloadDisabled => write!(
                f,
                "Wanted to download micromamba, but doing so is disabled by csm configuration"
            ),
            DownloadError::IO(e) => write!(f, "IO error: {}", e),
            DownloadError::Reqwest(e) => {
                write!(f, "Failed to download micromamba: {}", e)?;
                let mut source = e.source();
                while let Some(src) = source {
                    debug!("Caused by: {}", src);
                    source = src.source();
                }
                Ok(())
            }
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

/// Determine the download URL for micromamba
fn micromamba_url() -> Result<String, DownloadError> {
    let os_arch = if cfg!(target_os = "linux") {
        if cfg!(target_arch = "x86_64") {
            "linux-64"
        } else if cfg!(target_arch = "aarch64") {
            "linux-aarch64"
        } else {
            return Err(DownloadError::IncompatibleArch);
        }
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            "win-64"
        } else {
            return Err(DownloadError::IncompatibleArch);
        }
    } else {
        return Err(DownloadError::IncompatibleOS);
    };

    Ok(format!(
        "https://micro.mamba.pm/api/micromamba/{}/latest",
        os_arch
    ))
}

/// Write the executable from the micromamba archive, to the cache directory on
/// disk.
fn write_micromamba<R: Read>(
    config: &Config,
    mut entry: tar::Entry<R>,
) -> Result<PathBuf, DownloadError> {
    let micromamba_exe = micromamba_executable_name()?;
    let micromamba_path = csm_cache_dir(config)?.join(micromamba_exe);

    debug!(
        "Writing micromamba to disk at {}",
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

    Ok(micromamba_path)
}

/// Given a tar archive, find the `Entry` of the micromamba binary and return it
fn find_micromamba_in_archive<'a, R: Read>(
    tar_archive: &'a mut tar::Archive<R>,
) -> Result<tar::Entry<'a, R>, DownloadError> {
    let archive_binary_dir = if cfg!(target_os = "linux") {
        PathBuf::from("bin")
    } else if cfg!(target_os = "windows") {
        PathBuf::from("Library").join("bin")
    } else {
        return Err(DownloadError::IncompatibleOS);
    };

    let micromamba_exe = micromamba_executable_name()?;

    for entry in tar_archive.entries()? {
        let entry = entry?;
        if let Ok(path) = entry.path()
            && path == archive_binary_dir.join(micromamba_exe)
        {
            return Ok(entry);
        }
    }

    Err(DownloadError::BinNotInArchive)
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

    let url = micromamba_url()?;
    info!("micromamba was not found on path; downloading it now");
    debug!("Going to download {}", url);

    if !config.download_micromamba {
        return Err(DownloadError::DownloadDisabled);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .use_rustls_tls()
        .build()?;
    let response_tarbz2 = client.get(url).send()?;
    debug!("Download completed, sending it to BzDecoder");
    let bz2_decoder = bzip2::read::BzDecoder::new(response_tarbz2);
    let mut tar_archive = tar::Archive::new(bz2_decoder);

    debug!("Looking for bin/micromamba in the tarfile");
    let entry = find_micromamba_in_archive(&mut tar_archive)?;
    write_micromamba(config, entry)
}
