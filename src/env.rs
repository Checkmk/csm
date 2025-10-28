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

pub fn run(subcommand: Subcommand) {
    println!("{:?}", subcommand);
}
