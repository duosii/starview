use clap::Parser;
use starview_common::{enums::{AssetSize, DeviceType}, fs::write_file};
use starview_core::{download::{DownloadConfig, Downloader}, fetch::{FetchConfig, Fetcher}};

use crate::Error;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, value_enum, default_value_t = AssetSize::Full)]
    asset_size: AssetSize,

    #[arg(long)]
    asset_version: Option<String>,

    #[arg(long, short, value_enum, default_value_t = DeviceType::Android)]
    device: DeviceType,

    out_path: String,
}

pub async fn fetch_path(args: Args) -> Result<(), Error> {
    let config = FetchConfig::new(None, Some(args.device), None);
    let mut fetcher = Fetcher::new(config).await?;

    let (_, asset_paths) = fetcher.get_latest_asset_info().await?;

    let asset_paths = serde_json::to_vec_pretty(&asset_paths)?;
    write_file(&asset_paths, args.out_path).await?;

    Ok(())
}
