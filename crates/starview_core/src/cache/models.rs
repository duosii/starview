use std::{collections::HashSet, path::Path};

use serde::{Deserialize, Serialize};
use starview_common::{enums::DeviceType, fs::write_file};
use starview_net::models::{AssetPaths, AssetVersionInfo};
use tokio::{fs::File, io::AsyncReadExt};

use crate::error::FetchCacheError;

/// Cache that stores information related to the game server, such as user ID, asset paths, and more.
#[derive(Clone, Serialize, Deserialize)]
pub struct FetchCache {
    pub device_type: DeviceType,
    pub udid: String,
    pub version_info: Option<AssetVersionInfo>,
    pub asset_paths: Option<AssetPaths>,
    /// A hash set containing the sha256 of assets that have already been downloaded
    pub downloaded_asset_hashes: HashSet<String>
}

impl FetchCache {
    /// Creates a new FetchCache with the provided udid
    ///
    /// `version_info` and `asset_paths` will be None
    pub fn new(udid: String, device_type: DeviceType) -> Self {
        Self {
            udid,
            device_type,
            version_info: None,
            asset_paths: None,
            downloaded_asset_hashes: HashSet::new()
        }
    }

    /// Loads a FetchCache from the provided path
    pub async fn from_path(path: impl AsRef<Path>) -> Result<Self, FetchCacheError> {
        let mut cache_file = File::open(&path).await?;
        let cache_file_metadata = cache_file.metadata().await?;
        let mut file_bytes = Vec::with_capacity(cache_file_metadata.len().try_into()?);
        cache_file.read_to_end(&mut file_bytes).await?;
        let fetch_cache: Self = serde_json::from_slice(&file_bytes)?;
        Ok(fetch_cache)
    }

    /// Writes this FetchCache to a file at the specified path
    pub async fn write(&self, path: impl AsRef<Path>) -> Result<(), FetchCacheError> {
        let cache_bytes = serde_json::to_vec(self)?;
        write_file(&cache_bytes, path).await?;
        Ok(())
    }
}
