use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("patch error: {0}")]
    Patch(#[from] starview_patch::Error),
}
