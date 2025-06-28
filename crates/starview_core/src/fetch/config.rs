use std::path::PathBuf;

use starview_common::enums::DeviceType;
use url::Url;

const DEFAULT_CACHE_PATH: &str = "starview.cache";

/// Configuration for [`crate::fetch::Fetcher`]
pub struct FetchConfig {
    pub cache_path: PathBuf,
    pub device_type: Option<DeviceType>,
    pub api_host: Option<Url>,
}

impl FetchConfig {
    pub fn new(cache_path: Option<&str>, device_type: Option<DeviceType>, api_host: Option<Url>) -> Self {
        Self {
            cache_path: PathBuf::from(cache_path.unwrap_or(DEFAULT_CACHE_PATH)),
            device_type: device_type,
            api_host,
        }
    }
}
