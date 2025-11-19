use crate::env::CommonEnvArgs;

use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Common environment arguments
    #[command(flatten)]
    pub common: CommonEnvArgs,
    /// Path to a packed environment archive (ending in .tar.gz)
    #[arg(value_name = "ARCHIVE")]
    pub archive_path: PathBuf,
}
