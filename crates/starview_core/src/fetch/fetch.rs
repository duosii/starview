use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    str::FromStr,
};

use starview_common::{OptionalBuilder, enums::AssetSize};
use starview_net::{
    client::WafuriAPIClient,
    models::{AssetPaths, AssetVersionInfo},
};
use tokio::sync::watch;
use url::Url;

use crate::{cache::models::FetchCache, download::{DownloadConfig, Downloader}, error::FetchCacheError, fetch::{state::{FetchAssetInfoState, FetchState}, FetchConfig}, Error};

const DOWNLOAD_URL_STRIP_PREFIX: &str = "/patch/gf/upload_assets";

/// Interface for communicating with the game's API
pub struct Fetcher {
    state_sender: watch::Sender<FetchState>,
    client: WafuriAPIClient,
    cache_path: PathBuf,
    cache: FetchCache,
}

impl Fetcher {
    /// Initializes a new Fetcher with the provided config.
    pub async fn new(config: FetchConfig) -> Result<(Self, watch::Receiver<FetchState>), Error> {
        // get cache
        let cache = if let Ok(fetch_cache) = FetchCache::from_path(&config.cache_path).await {
            Some(fetch_cache)
        } else {
            None
        };

        // build client
        let client = if let Some(cache) = &cache {
            WafuriAPIClient::builder().uuid(cache.udid.clone())
        } else {
            WafuriAPIClient::builder()
        };

        let mut client = client
            .map(config.api_host, |builder, api_host| {
                builder.api_host(api_host)
            })
            .map(config.device_type, |builder, device_type| {
                builder.device_type(device_type)
            })
            .build()?;
        client.signup().await?;

        let (state_sender, recv) = watch::channel(FetchState::None);

        Ok((
            Self {
                state_sender,
                cache: cache.unwrap_or(FetchCache::new(
                    client.uuid.clone(),
                    client.device_type.clone(),
                )),
                cache_path: config.cache_path,
                client,
            },
            recv,
        ))
    }

    /// Writes the fetch cache to `self.cache_path`
    async fn write_cache(&self) -> Result<(), FetchCacheError> {
        self.cache.write(&self.cache_path).await
    }

    /// Fetches version info and asset paths from the game server for the provided `asset_version`.
    pub async fn get_asset_info(
        &mut self,
        asset_version: &str,
    ) -> Result<(AssetVersionInfo, AssetPaths), Error> {
        if let (Some(asset_paths), Some(version_info)) =
            (&self.cache.asset_paths, &self.cache.version_info)
        {
            // skip updating if:
            // - cache contains both asset_paths and version info
            // - and device type is the same as the client's device type
            if (asset_paths.info.client_asset_version == asset_version)
                && (self.cache.device_type == self.client.device_type)
            {
                self.state_sender.send_replace(FetchState::AssetInfo(FetchAssetInfoState::Finish));
                return Ok((version_info.clone(), asset_paths.clone()));
            }
        }

        // update cache by fetching the most recent asset paths & asset version info
        self.state_sender.send_replace(FetchState::AssetInfo(FetchAssetInfoState::GetAssetPaths));
        let mut asset_paths = self
            .client
            .get_asset_path(&asset_version, AssetSize::Full)
            .await?
            .ok_or(starview_net::Error::InvalidRequest(
                "could not load asset paths".into(),
            ))?;
        asset_paths.info.client_asset_version = asset_paths.info.target_asset_version.clone();

        self.state_sender.send_replace(FetchState::AssetInfo(FetchAssetInfoState::GetAssetVersionInfo));
        let asset_version_info = self
            .client
            .get_asset_version_info(&asset_version)
            .await?
            .ok_or(starview_net::Error::InvalidRequest(
                "could not load asset version info".into(),
            ))?;

        // update cache
        self.cache.asset_paths = Some(asset_paths.clone());
        self.cache.version_info = Some(asset_version_info.clone());
        self.write_cache().await?;

        self.state_sender.send_replace(FetchState::AssetInfo(FetchAssetInfoState::Finish));

        Ok((asset_version_info, asset_paths))
    }

    /// Fetches the latest version info and asset paths from the game server.
    pub async fn get_latest_asset_info(&mut self) -> Result<(AssetVersionInfo, AssetPaths), Error> {
        self.state_sender.send_replace(FetchState::AssetInfo(FetchAssetInfoState::GetAssetVersion));
        let available_asset_version = {
            let user_data =
                self.client
                    .load()
                    .await?
                    .ok_or(starview_net::Error::InvalidRequest(
                        "could not load player data".into(),
                    ))?;

            user_data.available_asset_version
        };

        self.get_asset_info(&available_asset_version).await
    }

    /// Inserts `url_str` into `url_hash_map` and `to_download_urls` if
    /// `hash` is not inside the provided `downloaded_asset_hashes` HashSet.
    ///
    /// Inserts `hash` into `new_downloaded_asset_hashes`
    /// if it was already in `downloaded_asset_hashes`
    fn insert_url_if_not_downloaded(
        hash: String,
        url_str: &str,
        downloaded_asset_hashes: &HashSet<String>,
        to_download_urls: &mut Vec<Url>,
        url_hash_map: &mut HashMap<Url, String>,
        new_downloaded_asset_hashes: &mut HashSet<String>,
    ) -> Result<(), url::ParseError> {
        if !downloaded_asset_hashes.contains(&hash) {
            let url = Url::from_str(url_str)?;
            url_hash_map.insert(url.clone(), hash);
            to_download_urls.push(url);
        } else {
            new_downloaded_asset_hashes.insert(hash);
        }

        Ok(())
    }

    /// Downloads the latest assets from the game server to the provided directory `out_path`
    pub async fn download_assets(&mut self, out_path: impl AsRef<Path>) -> Result<(), Error> {
        // confirm that out_path is a directory
        let out_path = out_path.as_ref();
        if !out_path.is_dir() {
            return Err(Error::NotDirectory(
                out_path.as_os_str().to_string_lossy().to_string(),
            ));
        }

        // extract info from FetchCache or get it from the game servers
        let (_, asset_paths) = self.get_latest_asset_info().await?;
        let downloaded_asset_hashes = &self.cache.downloaded_asset_hashes;

        // generate hashmap of urls to download
        let mut to_download_urls: Vec<Url> = Vec::new();
        let mut url_hash_map: HashMap<Url, String> = HashMap::new();
        let mut new_downloaded_asset_hashes: HashSet<String> = HashSet::new();

        for archive in asset_paths.full.archive {
            Self::insert_url_if_not_downloaded(
                archive.sha256,
                &archive.location,
                &downloaded_asset_hashes,
                &mut to_download_urls,
                &mut url_hash_map,
                &mut new_downloaded_asset_hashes,
            )?;
        }
        for diff in asset_paths.diff {
            for archive in diff.archive {
                Self::insert_url_if_not_downloaded(
                    archive.sha256,
                    &archive.location,
                    &downloaded_asset_hashes,
                    &mut to_download_urls,
                    &mut url_hash_map,
                    &mut new_downloaded_asset_hashes,
                )?;
            }
        }

        // create downloader
        let download_config = DownloadConfig::builder()
            .urls(to_download_urls)
            .out_path(out_path)
            .url_strip_prefix(DOWNLOAD_URL_STRIP_PREFIX.into())
            .build();
        let (downloader, recv) = Downloader::new(download_config);
        let (downloaded_urls, download_errors) = downloader.download().await?;

        // insert downloaded urls into new downloaded asset hashes hashset
        for downloaded_url in downloaded_urls {
            if let Some(hash) = url_hash_map.remove(&downloaded_url) {
                new_downloaded_asset_hashes.insert(hash);
            }
        }

        // replace downloaded asset hashes in cache & write
        self.cache.downloaded_asset_hashes = new_downloaded_asset_hashes;
        self.write_cache().await?;

        Ok(())
    }
}
