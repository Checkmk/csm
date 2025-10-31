//! Module for reading a user's ~/.csmrc, if it exists.

use log::debug;
use serde::Deserialize;
use std::default::Default;
use std::io::{Error, ErrorKind};

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Override the $MAMBA_ROOT_PREFIX when shelling out to micromamba.
    #[serde(default)]
    pub mamba_root_prefix: Option<String>,

    /// If true, don't make any changes or call any commands, just print what
    /// we *would* do normally.
    #[serde(default)]
    pub noop_mode: bool,

    /// Override the cache directory for testing purposes.
    #[serde(default)]
    pub cache_dir: Option<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Config {
            mamba_root_prefix: None,
            noop_mode: false,
            cache_dir: None,
        }
    }
}

impl Config {
    /// Read the user's ~/.csmrc if it exists, merging with the Default instance for
    /// Config. Return Err if a config file was found but failed to parse, otherwise
    /// Ok with the result of merging the config file values with the Default (and
    /// simply the Default if no config file exists).
    pub fn from_csmrc() -> Result<Self, std::io::Error> {
        let Some(home) = dirs::home_dir() else {
            return Ok(Self::default());
        };
        let csmrc_path = home.join(".csmrc");
        match std::fs::read_to_string(csmrc_path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                debug!("No .csmrc found, using defaults");
                Ok(Config::default())
            }
            Err(e) => Err(e),
            Ok(csmrc_data) => {
                let config = serde_yaml_ng::from_str(&csmrc_data)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e));
                debug!("config: {:?}", config);
                config
            }
        }
    }
}
