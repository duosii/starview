use std::path::Path;

use serde::{Deserialize, Serialize};
use starview_net::models::{AssetPaths, AssetVersionInfo};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{error::FetchCacheError, fs::write_file};

/// Cache that stores information related to the game server, such as user ID, asset paths, and more.
#[derive(Clone, Serialize, Deserialize)]
pub struct FetchCache {
    pub udid: String,
    pub version_info: Option<AssetVersionInfo>,
    pub asset_paths: Option<AssetPaths>,
}

impl FetchCache {

    /// Creates a new FetchCache with the provided udid
    /// 
    /// `version_info` and `asset_paths` will be None
    pub fn new(udid: String) -> Self {
        Self {
            udid,
            version_info: None,
            asset_paths: None,
        }
    }

    /// Loads a FetchCache from the provided path
    pub async fn from_path(path: impl AsRef<Path>) -> Result<Self, FetchCacheError> {
        let mut cache_file = File::open(path).await?;
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
