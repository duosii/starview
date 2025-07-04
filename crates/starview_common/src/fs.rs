use std::path::Path;

use tokio::{
    fs::{File, create_dir_all},
    io::AsyncWriteExt,
};

/// Writes the given bytes to the file at `path`
///
/// Overwrites and truncates existing files
pub async fn write_file(data: &[u8], path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    // write file
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).await?;
    }
    let mut out_file = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .await?;
    out_file.write_all(data).await?;
    Ok(())
}

