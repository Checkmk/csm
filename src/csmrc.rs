/// Module for reading a user's ~/.csmrc, if it exists.
use crate::util;

use log::debug;
use serde::Deserialize;
use std::default::Default;
use std::io::{Error, ErrorKind};

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Override the $MAMBA_ROOT_PREFIX when shelling out to micromamba.
    #[serde(default)]
    #[allow(dead_code)]
    pub mamba_root_prefix: Option<String>,
}

impl Config {
    /// Read the user's ~/.csmrc if it exists, merging with the Default instance for
    /// Config. Return Err if a config file was found but failed to parse, otherwise
    /// Ok with the result of merging the config file values with the Default (and
    /// simply the Default if no config file exists).
    pub fn from_csmrc() -> Result<Self, std::io::Error> {
        let Some(home) = util::homedir() else {
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
