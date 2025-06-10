use std::path::Path;

use crate::error::Error;

/// Ensures that, for a given path:
/// - It exists,
/// - It is a file,
/// - And its file name contains `expected_name`
pub fn validate_file_path(path: impl AsRef<Path>, expected_name: &str) -> Result<bool, Error> {
    let path = path.as_ref();

    let exists = path.try_exists()?;
    let name_correct = if let Some(file_name) = path.file_name() {
        file_name.to_string_lossy().contains(expected_name)
    } else {
        false
    };

    Ok(exists && name_correct && path.is_file())
}
