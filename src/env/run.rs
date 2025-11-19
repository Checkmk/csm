use crate::env::CommonArgs;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub common: CommonArgs,
    /// The command to run
    #[arg(value_name = "COMMAND")]
    pub command: String,
    /// Arguments to pass to the command
    #[arg(value_name = "ARGS")]
    pub arguments: Vec<String>,
}
