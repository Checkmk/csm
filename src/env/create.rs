use crate::env::CommonEnvArgs;

use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub common: CommonEnvArgs,

    /// If specified, overrides the post-creation setup file [default: robotmk-setup.yaml]
    #[arg(long = "setup-file", value_name = "PATH")]
    pub setup_file: Option<PathBuf>,
}
