use crate::env::CommonEnvArgs;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Common environment arguments
    #[command(flatten)]
    pub common: CommonEnvArgs,
}
