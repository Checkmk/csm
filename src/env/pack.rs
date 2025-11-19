use crate::env::CommonArgs;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Common environment arguments
    #[command(flatten)]
    pub common: CommonArgs,
    /// Output path/filename of the packed environment, ending in .tar.gz. If
    /// not specified, the same method is used to determine the environment name
    /// as for the "--name" parameter, and the default name is <env_name>.tar.gz
    #[arg(long, short, value_name = "OUTPUT")]
    pub output: Option<String>,
}
