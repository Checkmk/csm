use crate::csmrc::Config;

use log::debug;
use std::collections::HashMap;
use std::process::Command;

/// Return a [`Command`] ready to shell out to `micromamba` with the appropriate
/// environment variables set based on configuration.
pub fn micromamba(config: &Config, args: Vec<&str>) -> Command {
    let mut env_vars: HashMap<&str, String> = HashMap::new();

    if let Some(mamba_root_prefix) = &config.mamba_root_prefix {
        env_vars.insert("MAMBA_ROOT_PREFIX", mamba_root_prefix.to_string());
    }

    let mut cmd = Command::new("micromamba");
    cmd.args(args);
    cmd.envs(env_vars);
    debug!("About to run: {:?}", cmd);
    cmd
}
