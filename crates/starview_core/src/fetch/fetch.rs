use std::path::PathBuf;

use starview_common::{OptionalBuilder, enums::AssetSize};
use starview_net::{
    client::WafuriAPIClient,
    models::{AssetPaths, AssetVersionInfo},
};

use crate::{Error, cache::models::FetchCache, fetch::config::FetchConfig};

/// Interface for communicating with the game's API
pub struct Fetcher {
    client: WafuriAPIClient,
    cache_path: PathBuf,
    cache: FetchCache,
}

impl Fetcher {
    /// Initializes a new Fetcher with the provided config.
    pub async fn new(config: FetchConfig) -> Result<Self, Error> {
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

        Ok(Self {
            cache: cache.unwrap_or(FetchCache::new(client.uuid.clone())),
            cache_path: config.cache_path,
            client,
        })
    }

    /// Fetches version info and asset paths from the game server for the provided `asset_version`.
    pub async fn get_asset_info(
        &mut self,
        asset_version: &str,
    ) -> Result<(AssetVersionInfo, AssetPaths), Error> {
        if let (Some(asset_paths), Some(asset_version_info)) =
            (&self.cache.asset_paths, &self.cache.version_info)
        {
            // cache contains both asset_paths and version info
            if asset_paths.info.client_asset_version == asset_version {
                return Ok((asset_version_info.clone(), asset_paths.clone()));
            }
        }

        // cache doesn't contain asset_paths or asset_version info
        // or the cache was outdated
        let mut asset_paths = self
            .client
            .get_asset_path(&asset_version, AssetSize::Full)
            .await?
            .ok_or(starview_net::Error::InvalidRequest(
                "could not load asset paths".into(),
            ))?;
        asset_paths.info.client_asset_version = asset_paths.info.target_asset_version.clone();

        let asset_version_info = self
            .client
            .get_asset_version_info(&asset_version)
            .await?
            .ok_or(starview_net::Error::InvalidRequest(
                "could not load asset version info".into(),
            ))?;

        self.cache.asset_paths = Some(asset_paths.clone());
        self.cache.version_info = Some(asset_version_info.clone());

        self.cache.write(&self.cache_path).await?;

        Ok((asset_version_info, asset_paths))
    }

    /// Fetches the latest version info and asset paths from the game server.
    pub async fn get_latest_asset_info(&mut self) -> Result<(AssetVersionInfo, AssetPaths), Error> {
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
}
