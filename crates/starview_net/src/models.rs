use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DataHeaders {
    pub short_udid: u32,
    pub viewer_id: u32,
    pub servertime: u32,
    pub result_code: u8,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub udid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data_headers: DataHeaders,
    pub data: T,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    pub oaid: String,
    pub mac: String,
    pub media: String,
    pub os_er: String,
    pub android_id: String,
    pub storage_directory_path: String,
    pub channel_no: String,
    pub device_id: f32,
    pub termin_info: String,
}

impl Default for SignupRequest {
    fn default() -> Self {
        Self {
            oaid: "".into(),
            mac: "".into(),
            media: "none".into(),
            os_er: "".into(),
            android_id: "".into(),
            storage_directory_path:
                "/data/user/0/com.leiting.wf/com.leiting.wf/Local Store/custom_Release_Android"
                    .into(),
            channel_no: "".into(),
            device_id: 12489124124.0,
            termin_info: "".into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupResponse {
    #[serde(rename = "login_token")]
    pub login_token: String,
    pub new_account: u8,
    pub sign: Option<String>,
    pub create_date: Option<String>,
    pub role_name: Option<String>,
    pub role_id: Option<u32>,
    pub server_name: Option<String>,
    pub server_id: Option<String>,
    pub time_used: Option<u32>,
    pub account_name: Option<String>,
    pub login_mode: Option<u8>,
    pub login_type: Option<u8>,
    pub credit_account: Option<u8>,
    pub physical_value: Option<u8>,
    pub role_level: Option<u8>,
    pub ip: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoadRequest {
    pub oaid: String,
    pub viewer_id: u32,
    pub device_token: String,
    pub mac: String,
    pub imei: String,
    pub keychain: u32,
    pub graphics_device_name: String,
    pub storage_directory_path: String,
    pub platform_os_version: String,
    pub device_id: f32,
}

impl LoadRequest {
    pub fn from_viewer_id(viewer_id: u32) -> Self {
        Self {
            oaid: "".into(),
            viewer_id,
            device_token: "noDeviceToken".into(),
            mac: "".into(),
            imei: "none".into(),
            keychain: viewer_id,
            graphics_device_name: "OpenGL (Baseline Extended)".into(),
            storage_directory_path:
                "/data/user/0/com.leiting.wf/com.leiting.wf/Local Store/custom_Release_Android"
                    .into(),
            platform_os_version: "Android 12".into(),
            device_id: 12489124124.0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoadResponse {
    pub available_asset_version: String,
}

#[derive(Debug, Serialize)]
pub struct GetAssetPathRequest {
    pub target_asset_version: String,
    pub viewer_id: u32,
}

impl GetAssetPathRequest {
    pub fn new(target_asset_version: String, viewer_id: u32) -> Self {
        Self {
            target_asset_version,
            viewer_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetPathsInfo {
    pub client_asset_version: String,
    pub target_asset_version: String,
    pub eventual_target_asset_version: String,
    pub is_initial: bool,
    pub latest_maj_first_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetPathArchive {
    pub location: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetPathsFull {
    pub version: String,
    pub archive: Vec<AssetPathArchive>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetPathDiff {
    pub version: String,
    pub original_version: String,
    pub archive: Vec<AssetPathArchive>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetPaths {
    pub info: AssetPathsInfo,
    pub full: AssetPathsFull,
    pub diff: Vec<AssetPathDiff>,
    pub asset_version_hash: String,
}

impl AssetPaths {
    /// Merges two AssetPaths together
    ///
    /// Will only merge differences between the two
    pub fn extend(self, with: AssetPaths) -> Self {
        let mut full_archives_map: HashMap<String, AssetPathArchive> = HashMap::new();
        for archive in self
            .full
            .archive
            .into_iter()
            .chain(with.full.archive.into_iter())
        {
            full_archives_map
                .entry(archive.sha256.clone())
                .or_insert(archive);
        }

        let mut diff_map: HashMap<String, AssetPathsDiffMapEntry> = HashMap::new();
        for diff in self.diff.into_iter().chain(with.diff.into_iter()) {
            let diff_map_entry =
                diff_map
                    .entry(diff.version.clone())
                    .or_insert(AssetPathsDiffMapEntry::new(
                        diff.version.clone(),
                        diff.original_version.clone(),
                    ));
            for archive in diff.archive {
                diff_map_entry
                    .archive_map
                    .entry(archive.sha256.clone())
                    .or_insert(archive);
            }
        }

        Self {
            info: self.info,
            full: AssetPathsFull {
                version: self.full.version,
                archive: full_archives_map.into_values().collect(),
            },
            diff: diff_map.into_values().map(|entry| entry.into()).collect(),
            asset_version_hash: self.asset_version_hash,
        }
    }
}

/// Struct used when merging two AssetPaths together to keep track of diffs
struct AssetPathsDiffMapEntry {
    pub version: String,
    pub original_version: String,
    pub archive_map: HashMap<String, AssetPathArchive>,
}

impl AssetPathsDiffMapEntry {
    fn new(version: String, original_version: String) -> Self {
        Self {
            version,
            original_version,
            archive_map: HashMap::new(),
        }
    }
}

impl Into<AssetPathDiff> for AssetPathsDiffMapEntry {
    fn into(self) -> AssetPathDiff {
        AssetPathDiff {
            version: self.version,
            original_version: self.original_version,
            archive: self.archive_map.into_values().collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAssetVersionInfoRequest {
    pub asset_version: String,
    pub viewer_id: u32,
}

impl GetAssetVersionInfoRequest {
    pub fn new(asset_version: String, viewer_id: u32) -> Self {
        Self {
            asset_version,
            viewer_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetVersionInfo {
    pub base_url: String,
    pub files_list: String,
    pub total_size: u64,
    pub delayed_assets_size: u64,
}
