use std::{
    collections::{HashMap, HashSet},
    fs::create_dir_all,
    path::{Path, PathBuf},
    str::FromStr,
};

use starview_common::{OptionalBuilder, enums::AssetSize};
use starview_net::{
    client::WafuriAPIClient,
    models::{AssetPathArchive, AssetPaths, AssetVersionInfo},
};
use tokio::{join, sync::watch, try_join};
use url::Url;

use crate::{
    Error,
    cache::models::FetchCache,
    download::{DownloadConfig, Downloader, state::DownloadState},
    error::FetchCacheError,
    fetch::{
        FetchConfig,
        state::{DownloadAssetsState, DownloadFilesListState, FetchAssetInfoState, FetchState},
    },
};

const DOWNLOAD_URL_STRIP_PREFIX: &str = "/patch/gf/upload_assets";
const DOWNLOAD_FILES_LIST_URL_STRIP_PREFIX: &str = "/patch/gf/upload_assets/entities";

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
        let cache = FetchCache::from_path(&config.cache_path).await.ok();

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
                cache: cache.unwrap_or(FetchCache::new(client.uuid.clone(), client.device_type)),
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
    ) -> Result<(Vec<AssetVersionInfo>, AssetPaths), Error> {
        if let (Some(asset_paths), version_info) =
            (&self.cache.asset_paths, &self.cache.version_info)
        {
            // skip updating if:
            // - cache contains both asset_paths and version info
            // - and device type is the same as the client's device type
            if (asset_paths.info.client_asset_version == asset_version)
                && (self.cache.device_type == self.client.device_type)
            {
                self.state_sender
                    .send_replace(FetchState::AssetInfo(FetchAssetInfoState::Finish));
                return Ok((version_info.clone(), asset_paths.clone()));
            }
        }

        // update cache by fetching the most recent asset paths & asset version info
        self.state_sender
            .send_replace(FetchState::AssetInfo(FetchAssetInfoState::GetAssetInfo));
        let asset_paths_future = self.client.get_asset_path(asset_version, AssetSize::Full);
        let asset_version_info_future = self.client.get_asset_version_info(asset_version);

        if let (Some(mut asset_paths), asset_version_info) =
            try_join!(asset_paths_future, asset_version_info_future)?
        {
            asset_paths.info.client_asset_version = asset_paths.info.target_asset_version.clone();

            // update cache
            self.cache.asset_paths = Some(asset_paths.clone());
            self.cache.version_info = asset_version_info.clone();
            self.cache.device_type = self.client.device_type;
            self.write_cache().await?;

            self.state_sender
                .send_replace(FetchState::AssetInfo(FetchAssetInfoState::Finish));

            Ok((asset_version_info, asset_paths))
        } else {
            Err(Error::StarviewNet(starview_net::Error::InvalidRequest(
                "could not get asset paths or asset version info".into(),
            )))
        }
    }

    /// Fetches the latest version info and asset paths from the game server.
    pub async fn get_latest_asset_info(
        &mut self,
    ) -> Result<(Vec<AssetVersionInfo>, AssetPaths), Error> {
        self.state_sender
            .send_replace(FetchState::AssetInfo(FetchAssetInfoState::GetAssetVersion));
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
    ///
    /// Returns the number of bytes that should be downloaded
    fn insert_url_if_not_downloaded(
        archive: AssetPathArchive,
        downloaded_asset_hashes: &HashSet<String>,
        to_download_urls: &mut Vec<Url>,
        url_hash_map: &mut HashMap<Url, String>,
        new_downloaded_asset_hashes: &mut HashSet<String>,
    ) -> Result<u64, url::ParseError> {
        let hash = archive.sha256;
        if !downloaded_asset_hashes.contains(&hash) {
            let url = Url::from_str(&archive.location)?;
            url_hash_map.insert(url.clone(), hash);
            to_download_urls.push(url);
            Ok(archive.size)
        } else {
            new_downloaded_asset_hashes.insert(hash);
            Ok(0)
        }
    }

    /// Watches a DownloadState receiver for any changes,
    /// bridging them into a FetchState::DownloadAssets state update
    /// for this Fetcher.
    async fn bridge_download_state(
        mut download_recv: watch::Receiver<DownloadState>,
        state_sender: watch::Sender<FetchState>,
    ) {
        while download_recv.changed().await.is_ok() {
            let download_state = *download_recv.borrow_and_update();
            state_sender.send_replace(FetchState::DownloadAssets(DownloadAssetsState::Download(
                download_state,
            )));
            // break if the download state is Finish
            if download_state == DownloadState::Finish {
                break;
            }
        }
    }

    /// Downloads the latest assets from the game server to the provided directory `out_path`
    pub async fn download_assets(
        &mut self,
        out_path: impl AsRef<Path>,
        concurrency: usize,
    ) -> Result<(), Error> {
        validate_dir(&out_path)?;

        // extract info from FetchCache or get it from the game servers
        self.state_sender.send_replace(FetchState::DownloadAssets(
            DownloadAssetsState::FetchAssetInfo,
        ));
        let (_, asset_paths) = self.get_latest_asset_info().await?;
        let downloaded_asset_hashes = &self.cache.downloaded_asset_hashes;

        // generate hashmap of urls to download
        let mut to_download_urls: Vec<Url> = Vec::new();
        let mut url_hash_map: HashMap<Url, String> = HashMap::new();
        let mut new_downloaded_asset_hashes: HashSet<String> = HashSet::new();
        let mut total_bytes: u64 = 0;

        for archive in asset_paths.full.archive {
            total_bytes += Self::insert_url_if_not_downloaded(
                archive,
                downloaded_asset_hashes,
                &mut to_download_urls,
                &mut url_hash_map,
                &mut new_downloaded_asset_hashes,
            )?;
        }
        for diff in asset_paths.diff {
            for archive in diff.archive {
                total_bytes += Self::insert_url_if_not_downloaded(
                    archive,
                    downloaded_asset_hashes,
                    &mut to_download_urls,
                    &mut url_hash_map,
                    &mut new_downloaded_asset_hashes,
                )?;
            }
        }

        // send download start state with total download bytes
        self.state_sender.send_replace(FetchState::DownloadAssets(
            DownloadAssetsState::DownloadStart(total_bytes),
        ));

        // create downloader
        let download_config = DownloadConfig::builder()
            .urls(to_download_urls)
            .out_path(out_path)
            .url_strip_prefix(DOWNLOAD_URL_STRIP_PREFIX.into())
            .concurrency(concurrency)
            .build();
        let (downloader, recv) = Downloader::new(download_config);

        // listen to the downloader state recv
        // and bridge to FetchState
        let watch_future = Self::bridge_download_state(recv, self.state_sender.clone());
        let download_future = downloader.download();

        // join download futures
        let (_, download_result) = join!(watch_future, download_future);
        let (downloaded_urls, _) = download_result?;

        // insert downloaded urls into new downloaded asset hashes hashset
        for downloaded_url in downloaded_urls {
            if let Some(hash) = url_hash_map.remove(&downloaded_url) {
                new_downloaded_asset_hashes.insert(hash);
            }
        }

        // replace downloaded asset hashes in cache & write
        self.cache.downloaded_asset_hashes = new_downloaded_asset_hashes;
        self.write_cache().await?;
        self.state_sender
            .send_replace(FetchState::DownloadAssets(DownloadAssetsState::Finish));

        Ok(())
    }

    /// Downloads file list CSVs to the provided `out_path`.
    ///
    /// A maximum of two files will be downloaded
    /// depending on the DeviceType provided to this fetcher
    pub async fn download_files_list(&mut self, out_path: impl AsRef<Path>) -> Result<(), Error> {
        validate_dir(&out_path)?;

        self.state_sender
            .send_replace(FetchState::DownloadFilesList(
                DownloadFilesListState::FetchAssetInfo,
            ));
        let (asset_version_info, _) = self.get_latest_asset_info().await?;

        let mut to_download_urls: Vec<Url> = Vec::new();
        for info in asset_version_info.iter().take(2) {
            let url = Url::from_str(&info.files_list)?;
            to_download_urls.push(url);
        }

        // create downloader
        self.state_sender
            .send_replace(FetchState::DownloadFilesList(
                DownloadFilesListState::DownloadStart(to_download_urls.len().try_into().unwrap()),
            ));
        let download_config = DownloadConfig::builder()
            .urls(to_download_urls)
            .out_path(out_path)
            .url_strip_prefix(DOWNLOAD_FILES_LIST_URL_STRIP_PREFIX.into())
            .concurrency(2)
            .build();
        let (downloader, recv) = Downloader::new(download_config);

        // listen to the downloader state recv
        // and bridge to FetchState
        let watch_future = Self::bridge_download_state(recv, self.state_sender.clone());
        let download_future = downloader.download();

        // join download futures
        let (_, download_result) = join!(watch_future, download_future);
        download_result?;

        self.state_sender
            .send_replace(FetchState::DownloadFilesList(
                DownloadFilesListState::Finish,
            ));

        Ok(())
    }
}

fn validate_dir(dir_path: impl AsRef<Path>) -> Result<(), Error> {
    // confirm that out_path is a directory
    let dir_path = dir_path.as_ref();
    if !dir_path.is_dir() {
        // create path to directory if it doesn't exist
        if dir_path.try_exists()? {
            return Err(Error::NotDirectory(
                dir_path.as_os_str().to_string_lossy().to_string(),
            ));
        } else {
            create_dir_all(dir_path)?;
        }
    }
    Ok(())
}
