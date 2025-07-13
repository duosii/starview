mod assets;
mod path;

use clap::{Args, Subcommand};

use crate::Error;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Fetch a file that gives information about the game's assets
    Path(path::Args),
    /// Fetches the game's assets
    Assets(assets::Args),
}

#[derive(Debug, Args)]
pub struct FetchArgs {
    #[command(subcommand)]
    command: Commands,
}

pub async fn fetch(args: FetchArgs) -> Result<(), Error> {
    match args.command {
        Commands::Path(args) => path::fetch_path(args).await,
        Commands::Assets(args) => assets::fetch_assets(args).await,
    }
}
