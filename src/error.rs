use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("error when walking directory: {0}")]
    WallkDir(#[from] walkdir::Error),

    #[error("error when stripping path prefix: {0}")]
    StripPathPrefix(#[from] std::path::StripPrefixError),

    #[error("error when converting integer type: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),

    #[error("error when parsing patch: {0}")]
    PatchParse(String),

    #[error("attempt to patch a file that does not exist: {0}")]
    ToPatchFileMissing(String),

    #[error("could not find FFDec's install location.")]
    FFDecPath(),

    #[error("error when extracting flash scripts from APK: {0}")]
    FFDecExtract(String),

    #[error("error when importing flash scripts: {0}")]
    FFDecImport(String),

    #[error("could not find apksigner's install location.")]
    ApkSignerPath(),

    #[error("error when signing APK: {0}")]
    Sign(String),

    #[error("path is not a directory: {0}")]
    NotDirectory(String),
}
