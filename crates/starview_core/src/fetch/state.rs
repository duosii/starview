use crate::download::state::DownloadState;

/// The state of a fetch asset info task
#[derive(Clone, Copy, Debug)]
pub enum FetchAssetInfoState {
    /// The most recent asset version is being queried from the server
    GetAssetVersion,
    /// The asset paths and version info are being acquired from the server
    GetAssetInfo,
    /// The asset info has been successfully retrieved
    Finish,
}

/// The state of an asset download
#[derive(Clone, Copy, Debug)]
pub enum DownloadAssetsState {
    /// Asset info is being retrieved
    FetchAssetInfo,
    /// The provided number of bytes will be downloaded
    DownloadStart(u64),
    /// A download state update
    Download(DownloadState),
    /// The assets download process has completed
    Finish,
}

/// The current state of a [`crate::fetch::Fetcher`]
#[derive(Clone, Copy, Debug)]
pub enum FetchState {
    None,
    AssetInfo(FetchAssetInfoState),
    DownloadAssets(DownloadAssetsState),
}
