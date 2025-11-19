use crate::csmrc::Config;

use std::fs::{DirBuilder, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

const DOT_GITIGNORE: &[u8] = include_bytes!("../../robot-skel/.gitignore");
const ROBOTMK_ENV: &[u8] = include_bytes!("../../robot-skel/robotmk-env.yaml");
const ROBOTMK_SETUP: &[u8] = include_bytes!("../../robot-skel/robotmk-setup.yaml");
const ROBOT_TOML: &[u8] = include_bytes!("../../robot-skel/robot.toml");
const DOT_ROBOT_TOML: &[u8] = include_bytes!("../../robot-skel/.robot.toml");
const SAMPLE_ROBOT: &[u8] = include_bytes!("../../robot-skel/sample.robot");

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Create a Robotmk robot
    Create(CreateArgs),

    /// Run a Robotmk robot
    Run,
}

#[derive(Debug, clap::Args)]
pub struct CreateArgs {
    /// Directory path at which to create the robot
    path: String,
}

/// Copy the default, minimal template to the specified (directory) path.
fn copy_minimal_template(to: &Path) -> io::Result<()> {
    if to.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("The path {} already exists", to.to_string_lossy()),
        ));
    }
    DirBuilder::new().recursive(true).create(to)?;
    for (text, filename) in [
        (DOT_GITIGNORE, ".gitignore"),
        (ROBOTMK_ENV, "robotmk-env.yaml"),
        (ROBOTMK_SETUP, "robotmk-setup.yaml"),
        (ROBOT_TOML, "robot.toml"),
        (DOT_ROBOT_TOML, ".robot.toml"),
        (SAMPLE_ROBOT, "sample.robot"),
    ] {
        let path = to.join(filename);
        let fh = File::create(path)?;
        let mut writer = BufWriter::new(fh);
        writer.write_all(text)?;
        writer.flush()?;
    }
    Ok(())
}

pub fn run(config: Config, subcommand: Subcommand) -> io::Result<()> {
    match subcommand {
        Subcommand::Create(args) => copy_minimal_template(&PathBuf::from(args.path)),
        _ => {
            println!("{:?}", config);
            println!("{:?}", subcommand);
            Ok(())
        }
    }
}
