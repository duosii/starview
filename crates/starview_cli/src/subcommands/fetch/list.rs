use clap::Parser;
use starview_common::enums::DeviceType;
use starview_core::{
    download::state::DownloadState,
    fetch::{
        FetchConfig, Fetcher,
        state::{DownloadFilesListState, FetchState},
    },
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

    /// The device type that lists will be acquired for
    #[arg(long, short, value_enum, default_value_t = DeviceType::All)]
    device: DeviceType,

    /// Path to the starview cache,
    /// "starview.cache" by default
    #[arg(long, short, value_enum)]
    cache_path: Option<String>,

    /// Path to the directory where lists will be downloaded
    out_path: String,
}

/// Watches a FetchState [`tokio::sync::watch::Receiver`] for any updates,
/// printing status to the console.
async fn watch_fetch_state(mut recv: watch::Receiver<FetchState>) {
    let mut progress_bar: Option<indicatif::ProgressBar> = None;

    while recv.changed().await.is_ok() {
        let fetch_state = *recv.borrow_and_update();
        if let FetchState::DownloadFilesList(state) = fetch_state {
            match state {
                DownloadFilesListState::FetchAssetInfo => {
                    println!(
                        "{}[1/2] {}Getting asset information...",
                        color::TEXT_VARIANT.render_fg(),
                        color::TEXT.render_fg()
                    );
                }
                DownloadFilesListState::DownloadStart(file_count) => {
                    println!(
                        "{}[2/2] {}Downloading files lists...",
                        color::TEXT_VARIANT.render_fg(),
                        color::TEXT.render_fg()
                    );
                    progress_bar = Some(ProgressBar::progress(file_count));
                }
                DownloadFilesListState::Download(download_state) => {
                    if let DownloadState::FileDownload(file_size) = download_state {
                        if let Some(progress) = &progress_bar {
                            progress.inc(file_size);
                        }
                    }
                }
                DownloadFilesListState::Finish => {
                    progress_bar.finish_and_clear();
                    break;
                }
            }
        }
    }
}

pub async fn fetch_files_list(args: Args) -> Result<(), Error> {
    let fetch_start_instant = Instant::now();
    let config = FetchConfig::new(args.cache_path, Some(args.device), None);
    let (mut fetcher, recv) = Fetcher::new(config).await?;

    let state_watcher = if args.quiet {
        None
    } else {
        Some(tokio::spawn(watch_fetch_state(recv)))
    };

    fetcher.download_files_list(&args.out_path).await?;

    if let Some(watcher) = state_watcher {
        watcher.await?;
        println!(
            "{}Successfully files lists to '{}' in {:?}.{}",
            color::SUCCESS.render_fg(),
            args.out_path,
            Instant::now().duration_since(fetch_start_instant),
            color::TEXT.render_fg()
        )
    }

    Ok(())
}
