use std::path::PathBuf;

use clap::Parser;
use starview_common::enums::AssetSize;
use starview_core::fetch::{FetchConfig, Fetcher};
use starview_net::client::WafuriAPIClient;
use tokio::{
    fs::{File, create_dir_all},
    io::AsyncWriteExt,
};

use crate::Error;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, value_enum, default_value_t = AssetSize::Full)]
    asset_size: AssetSize,

    #[arg(long)]
    asset_version: Option<String>,

    out_path: String,
}

pub async fn fetch_path(args: Args) -> Result<(), Error> {
    let config = FetchConfig::new(None, None, None);
    let mut fetcher = Fetcher::new(config).await?;

    fetcher.get_latest_asset_info().await?;
    // let mut client = WafuriAPIClient::builder().build()?;
    // client.signup().await?;

    // let asset_version = if let Some(asset_version) = args.asset_version {
    //     asset_version
    // } else {
    //     let player_data = client.load().await?.unwrap();
    //     player_data.available_asset_version
    // };

    // let paths = client.get_asset_path(&asset_version, args.asset_size).await?.unwrap();

    // // write file
    // let out_path = PathBuf::from(args.out_path);
    // if let Some(parent) = out_path.parent() {
    //     create_dir_all(parent).await?;
    // }
    // let mut out_file = File::options()
    //     .write(true)
    //     .create(true)
    //     .truncate(true)
    //     .open(out_path)
    //     .await?;
    // out_file.write_all(&serde_json::to_vec_pretty(&paths).unwrap()).await?;

    Ok(())
}
