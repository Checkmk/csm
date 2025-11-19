use serde::Deserialize;
use std::io::{Error, ErrorKind};
use std::path::Path;

/// Contains the fields we need from a parsed environment file.
#[derive(Deserialize)]
pub struct RobotmkEnv {
    /// The name of the environment
    pub name: Option<String>,
}

/// Attempt to parse an environment file.
pub fn parse_env_yaml<P: AsRef<Path>>(path: P) -> Result<RobotmkEnv, std::io::Error> {
    let contents = std::fs::read_to_string(path)?;
    serde_yaml_ng::from_str(&contents).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}
