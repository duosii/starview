use clap::Parser;
use starview_common::enums::DeviceType;
use starview_core::{
    download::state::DownloadState,
    fetch::{
        FetchConfig, Fetcher,
        state::{DownloadAssetsState, FetchState},
    },
};
use tokio::{sync::watch, time::Instant};

use crate::{Error, color, progress::ProgressBar};

#[derive(Parser, Debug)]
pub struct Args {
    /// If status messages should be displayed
    #[arg(long, short, default_value_t = false)]
    quiet: bool,

    /// The version of the assets,
    /// uses the latset version by default
    #[arg(long)]
    asset_version: Option<String>,

    /// The device type that assets will be acquired for
    #[arg(long, short, value_enum, default_value_t = DeviceType::All)]
    device: DeviceType,

    /// Path to the starview cache,
    /// "starview.cache" by default
    #[arg(long)]
    cache_path: Option<String>,

    /// The maximum number of files to download at once
    #[arg(long, short, default_value_t = 5)]
    concurrency: usize,

    /// Path to the directory where assets will be downloaded
    out_path: String,
}

/// Watches a FetchState [`tokio::sync::watch::Receiver`] for any updates,
/// printing status to the console.
async fn watch_fetch_state(mut recv: watch::Receiver<FetchState>) {
    let mut progress_bar: Option<indicatif::ProgressBar> = None;

    while recv.changed().await.is_ok() {
        let fetch_state = *recv.borrow_and_update();
        if let FetchState::DownloadAssets(state) = fetch_state {
            match state {
                DownloadAssetsState::FetchAssetInfo => {
                    println!(
                        "{}[1/2] {}Getting asset information...",
                        color::TEXT_VARIANT.render_fg(),
                        color::TEXT.render_fg()
                    );
                }
                DownloadAssetsState::DownloadStart(total_bytes) => {
                    println!(
                        "{}[2/2] {}Downloading assets...",
                        color::TEXT_VARIANT.render_fg(),
                        color::TEXT.render_fg()
                    );
                    progress_bar = Some(ProgressBar::download(total_bytes));
                }
                DownloadAssetsState::Download(download_state) => {
                    if let DownloadState::FileDownload(file_size) = download_state {
                        if let Some(progress) = &progress_bar {
                            progress.inc(file_size);
                        }
                    }
                }
                DownloadAssetsState::Finish => {
                    if let Some(progress) = &progress_bar {
                        progress.finish_and_clear();
                    }
                    break;
                }
            }
        }
    }
}

pub async fn fetch_assets(args: Args) -> Result<(), Error> {
    let fetch_start_instant = Instant::now();
    let config = FetchConfig::new(args.cache_path, Some(args.device), None);
    let (mut fetcher, state_recv) = Fetcher::new(config).await?;

    let state_watcher = if args.quiet {
        None
    } else {
        Some(tokio::spawn(watch_fetch_state(state_recv)))
    };

    fetcher
        .download_assets(&args.out_path, args.concurrency)
        .await?;

    if let Some(watcher) = state_watcher {
        watcher.await?;
        println!(
            "{}Successfully downloaded assets to '{}' in {:?}.{}",
            color::SUCCESS.render_fg(),
            args.out_path,
            Instant::now().duration_since(fetch_start_instant),
            color::TEXT.render_fg()
        )
    }

    Ok(())
}
