mod assets;
mod list;
mod path;

use clap::{Args, Subcommand};

use crate::Error;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Fetch a file that gives information about the game's assets
    Path(path::Args),
    /// Fetches the game's assets
    Assets(assets::Args),
    /// Fetches files lists
    List(list::Args),
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
        Commands::List(args) => list::fetch_files_list(args).await,
    }
}
