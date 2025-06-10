mod color;
mod error;
mod subcommands;

use clap::{Parser, Subcommand};

use crate::{color::get_clap_styles, subcommands::patch};

pub use error::Error;

#[derive(Debug, Subcommand)]
enum Commands {
    /// Patch the game's APK
    Patch(patch::Args),
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None, styles=get_clap_styles())]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

pub fn run() -> Result<(), clap::Error> {
    let cli = Cli::try_parse()?;

    let command_result = match cli.command {
        Commands::Patch(args) => patch::patch(args),
    };

    if let Err(err) = command_result {
        println!(
            "{}{}{}",
            color::ERROR.render_fg(),
            err,
            color::TEXT.render_fg()
        )
    }

    Ok(())
}
