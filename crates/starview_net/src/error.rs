use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("request error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("invalid header value: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("msgpack encode error: {0}")]
    RmpSerdeEncode(#[from] rmp_serde::encode::Error),

    #[error("msgpack decode error: {0}")]
    RmpSerdeDecode(#[from] rmp_serde::decode::Error),

    #[error("error when decoding base64")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("invalid network request: {0}")]
    InvalidRequest(String),
}
