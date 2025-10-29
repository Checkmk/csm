use crate::csmrc::Config;

use log::{debug, error};
use serde::Deserialize;
use std::io::{Error, ErrorKind};
use std::path::Component;
use std::process;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Create an environment
    Create(CreateArgs),
    /// Activate an environment
    Activate,
    /// Deactivate an environment
    Deactivate,
    /// Run an executable in an environment
    Run,
    /// ???
    Pack,
    /// ???
    Unpack,
    /// List existing environments
    List,
    /// Display information about the micromamba setup
    Info,
}

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// If specified, the name of the environment. If not specified, csm will
    /// look to robotmk-env.yaml for a "name" field to use instead. As a last
    /// resort, the current directory name will be used
    #[arg(short, long)]
    name: Option<String>,
}

/// Contains the fields we need from a parsed `robotmk-env.yml` file.
#[derive(Deserialize)]
struct RobotmkEnv {
    /// The name of the environment
    name: Option<String>,
}

/// Attempt to parse a robotmk-env.yaml in the current directory.
fn parse_robotmk_env_yaml() -> Result<RobotmkEnv, std::io::Error> {
    // TODO: Should we handle .yml too?
    let contents = std::fs::read_to_string("robotmk-env.yaml")?;
    serde_yaml_ng::from_str(&contents).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

pub fn determine_env_name(args: CreateArgs) -> Option<String> {
    // If someone gave an explicit --name, use that first.
    if let Some(name) = args.name {
        debug!("Using '{}' as env name, given by CLI argument", name);
        return Some(name);
    }

    // Fallback 1: Look for a name key in robotmk-env.yaml
    // We ignore errors from parse_robotmk_env_yaml() here, we'll fall back
    // below if we can't parse it for some reason
    if let Ok(env) = parse_robotmk_env_yaml()
        && let Some(name) = env.name
    {
        debug!("Using '{}' as env name, found in robotmk-env.yaml", name);
        return Some(name);
    }

    // Fallback 2: Current directory name
    match std::env::current_dir() {
        Err(e) => {
            debug!("Could not determine current directory: {}", e);
            None
        }
        Ok(pathbuf) => match pathbuf.components().next_back() {
            Some(Component::Normal(s)) => match s.to_str().map(String::from) {
                Some(name) => {
                    debug!(
                        "Using '{}' as env name, taken from current directory name",
                        name
                    );
                    Some(name)
                }
                _ => None, // Likely could not convert path name to utf-8
            },
            _ => None, // In theory, I think this should never happen
        },
    }
}

pub fn run(config: Config, subcommand: Subcommand) {
    match subcommand {
        Subcommand::Create(args) => {
            let Some(env_name) = determine_env_name(args) else {
                error!("No environment name could be determined. You can specify one with --name");
                process::exit(1); // TODO: Probably better to return Result and let main() do this.
            };
            println!("env: {}", env_name);
        }
        _ => {
            println!("{:?}", config);
            println!("{:?}", subcommand);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    /// Run a test in a temporary directory with an optional robotmk-env.yaml in it
    fn run_in_temp_dir<F>(dir_name: &str, yaml_content: Option<&str>, test_fn: F)
    where
        F: FnOnce(),
    {
        let temp_dir = env::temp_dir().join(dir_name);
        fs::create_dir_all(&temp_dir).unwrap();

        if let Some(content) = yaml_content {
            let yaml_path = temp_dir.join("robotmk-env.yaml");
            fs::write(&yaml_path, content).unwrap();
        }

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        test_fn();

        env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_determine_env_name_with_cli_arg() {
        let args = CreateArgs {
            name: Some("test-env".to_string()),
        };

        let result = determine_env_name(args);
        assert_eq!(result, Some("test-env".to_string()));
    }

    #[test]
    fn test_determine_env_name_cli_arg_overrides_yaml() {
        run_in_temp_dir("csm_test_override", Some("name: yaml-env-name"), || {
            let args = CreateArgs {
                name: Some("cli-override".to_string()),
            };

            let result = determine_env_name(args);
            assert_eq!(result, Some("cli-override".to_string()));
        });
    }

    #[test]
    fn test_determine_env_name_robotmk_env_yaml() {
        // (dir_name, yaml, expected)
        let test_cases = vec![
            (
                "valid_yaml",
                Some("name: yaml-env-name\nother_field: value"),
                "yaml-env-name",
            ),
            (
                "yaml_no_name",
                Some("other_field: value\nyet_another: field"),
                "yaml_no_name",
            ),
            (
                "invalid_yaml",
                Some("invalid: yaml: content: \"unclosed"),
                "invalid_yaml",
            ),
            ("no_yaml", None, "no_yaml"),
        ];

        for (dir_name, yaml, expected) in test_cases {
            run_in_temp_dir(dir_name, yaml, || {
                let args = CreateArgs { name: None };
                let result = determine_env_name(args);
                assert_eq!(result.unwrap(), expected, "Failed case: {}", dir_name);
            });
        }
    }
}
