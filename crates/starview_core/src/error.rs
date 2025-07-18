use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("fetch cache error: {0}")]
    FetchCache(#[from] FetchCacheError),

    #[error("starview network error: {0}")]
    StarviewNet(#[from] starview_net::Error),

    #[error("reqwest network error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("error when converting integer type: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),

    #[error("error when parsing string as url: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("provided path '{0}' is not a directory")]
    NotDirectory(String),
}

#[derive(Debug, Error)]
pub enum FetchCacheError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("error when parsing fetch cache file: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("error when converting integer type: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),
}
