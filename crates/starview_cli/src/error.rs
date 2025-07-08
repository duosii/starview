use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("patch error: {0}")]
    StarviewPatch(#[from] starview_patch::Error),

    #[error("game API error: {0}")]
    StarviewNet(#[from] starview_net::Error),

    #[error("core error: {0}")]
    StarviewCore(#[from] starview_core::Error),

    #[error("serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("error when joining threads: {0}")]
    TokioJoin(#[from] tokio::task::JoinError),
}
