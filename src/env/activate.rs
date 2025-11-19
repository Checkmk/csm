use crate::env::CommonArgs;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Common environment arguments
    #[command(flatten)]
    pub common: CommonArgs,
}
