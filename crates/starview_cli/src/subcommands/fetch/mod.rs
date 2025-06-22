mod path;

use clap::{Subcommand, Args};

use crate::Error;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Fetch a file that gives information about the game's assets
    Path(path::Args),
}

#[derive(Debug, Args)]
pub struct FetchArgs {
    #[command(subcommand)]
    command: Commands,
}

pub async fn fetch(args: FetchArgs) -> Result<(), Error> {
    match args.command {
        Commands::Path(args) => path::fetch_path(args),
    }.await
}