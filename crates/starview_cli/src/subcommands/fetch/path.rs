use clap::Parser;
use starview_common::{enums::DeviceType, fs::write_file};
use starview_core::fetch::{
    FetchConfig, Fetcher,
    state::{FetchAssetInfoState, FetchState},
};
use tokio::{sync::watch, time::Instant};

use crate::{
    Error, color,
    progress::{FinishAndClear, ProgressBar},
};

#[derive(Parser, Debug)]
pub struct Args {
    /// If status messages should be displayed
    #[arg(long, short, default_value_t = false)]
    quiet: bool,

    /// The version of the assets,
    /// uses the latset version by default
    #[arg(long)]
    asset_version: Option<String>,

    /// The device type that paths will be acquired for
    #[arg(long, short, value_enum, default_value_t = DeviceType::All)]
    device: DeviceType,

    /// Path to the starview cache,
    /// "starview.cache" by default
    #[arg(long, short, value_enum)]
    cache_path: Option<String>,

    /// Where to output the paths file
    out_path: String,
}

/// Watches a FetchState [`tokio::sync::watch::Receiver`] for any updates,
/// printing status to the console.
async fn watch_fetch_state(mut recv: watch::Receiver<FetchState>) {
    let mut progress_bar: Option<indicatif::ProgressBar> = None;

    while recv.changed().await.is_ok() {
        let fetch_state = *recv.borrow_and_update();
        if let FetchState::AssetInfo(state) = fetch_state {
            match state {
                FetchAssetInfoState::GetAssetVersion => {
                    println!(
                        "{}[1/2] {}Getting most recent asset version...",
                        color::TEXT_VARIANT.render_fg(),
                        color::TEXT.render_fg()
                    );
                    progress_bar = Some(ProgressBar::spinner());
                }
                FetchAssetInfoState::GetAssetInfo => {
                    progress_bar.finish_and_clear();
                    println!(
                        "{}[2/2] {}Downloading asset info...",
                        color::TEXT_VARIANT.render_fg(),
                        color::TEXT.render_fg()
                    );
                    progress_bar = Some(ProgressBar::spinner());
                }
                FetchAssetInfoState::Finish => {
                    progress_bar.finish_and_clear();
                    break;
                }
            }
        }
    }
}

pub async fn fetch_path(args: Args) -> Result<(), Error> {
    let fetch_start_instant = Instant::now();
    let config = FetchConfig::new(args.cache_path, Some(args.device), None);
    let (mut fetcher, state_recv) = Fetcher::new(config).await?;

    let state_watcher = if args.quiet {
        None
    } else {
        Some(tokio::spawn(watch_fetch_state(state_recv)))
    };

    let (_, asset_paths) = fetcher.get_latest_asset_info().await?;

    let asset_paths = serde_json::to_vec_pretty(&asset_paths)?;
    write_file(&asset_paths, &args.out_path).await?;

    if let Some(watcher) = state_watcher {
        watcher.await?;
        println!(
            "{}Successfully wrote asset paths to '{}' in {:?}.{}",
            color::SUCCESS.render_fg(),
            args.out_path,
            Instant::now().duration_since(fetch_start_instant),
            color::TEXT.render_fg()
        )
    }

    Ok(())
}
