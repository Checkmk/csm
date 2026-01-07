//! Module for reading a user's ~/.csmrc, if it exists.

use log::debug;
use serde::Deserialize;
use std::default::Default;
use std::fs::File;
use std::io::{Error, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EnvCreateConfig {
    /// If true (default), verify SSL/TLS certificates. This is used for
    /// configuring passthrough to micromamba (and indirectly, pip).
    pub ssl_verify: bool,

    /// The SSL/TLS certificate bundle to verify with. When None (default), uses
    /// the default, which when calling out to micromamba, corresponds to the
    /// system chain or "ca-certificates" from conda-forge if installed.
    pub ssl_bundle: Option<String>,

    /// Disable SSL/TLS revokation checking (affects passthrough to micromamba).
    /// This is equivalent to passing --ssl-no-revoke to micromamba.
    pub ssl_no_revoke: bool,
}

impl Default for EnvCreateConfig {
    fn default() -> Self {
        EnvCreateConfig {
            ssl_verify: true,
            ssl_bundle: None,
            ssl_no_revoke: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Override the $MAMBA_ROOT_PREFIX when shelling out to micromamba.
    pub mamba_root_prefix: Option<String>,

    /// If true, don't make any changes or call any commands, just print what
    /// we *would* do normally.
    pub noop_mode: bool,

    /// Override the cache directory for testing purposes.
    pub cache_dir: Option<String>,

    /// If false, skip downloading micromamba even if needed (for testing).
    pub download_micromamba: bool,

    /// (Internal) If true, the program is being run in verbose mode.
    /// We do not support this being set from the configuration file, because
    /// the configuration file is parsed after logging is initialized. The user
    /// can technically do it, we won't error, but it won't have much effect.
    pub verbose: bool,

    /// If true, skip checking for LongPaths support on Windows, and never try
    /// to set it.
    pub skip_longpaths_check: bool,

    /// Options for the 'env create' subcommand
    pub env_create: EnvCreateConfig,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Config {
            mamba_root_prefix: None,
            noop_mode: false,
            cache_dir: None,
            download_micromamba: true,
            verbose: false,
            skip_longpaths_check: false,
            env_create: EnvCreateConfig::default(),
        }
    }
}

impl Config {
    /// Validate the configuration to check for common errors.
    fn validate(self) -> Result<Self, std::io::Error> {
        // Specifying a bundle when ssl_verify is false makes no sense.
        // Do the safe thing and bail out - they might have meant to use the
        // bundle and we should not just disable verification in this case.
        if !self.env_create.ssl_verify && self.env_create.ssl_bundle.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "SSL/TLS verification was disabled, but a bundle was still specified",
            ));
        }
        Ok(self)
    }

    /// Read the user's ~/.csmrc if it exists, merging with the Default instance for
    /// Config. Return Err if a config file was found but failed to parse, otherwise
    /// Ok with the result of merging the config file values with the Default (and
    /// simply the Default if no config file exists).
    pub fn from_csmrc() -> Result<Self, std::io::Error> {
        fn parse_or_io_error(f: File) -> Result<Config, std::io::Error> {
            match serde_yaml_ng::from_reader::<File, Config>(f) {
                Ok(cfg) => cfg.validate(),
                Err(e) => Err(Error::new(ErrorKind::InvalidData, e)),
            }
        }

        let Some(home) = std::env::home_dir() else {
            debug!("Could not determine home directory to read .csmrc, using defaults");
            return Ok(Self::default());
        };

        let csmrc_path = home.join(".csmrc");
        File::open(csmrc_path)
            .and_then(parse_or_io_error)
            .or_else(|e| {
                if e.kind() == ErrorKind::NotFound {
                    debug!("No .csmrc found, using defaults");
                    Ok(Config::default())
                } else {
                    Err(e)
                }
            })
    }
}
