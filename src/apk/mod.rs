pub mod signer;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use tempfile::TempDir;
use walkdir::WalkDir;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::error::Error;

/// The location of the apk's .swf file relative to the APK directory
pub const DEFAULT_WF_SWF_LOCATION: &str = "assets/worldflipper_android_release.swf";

/// What compression method will be used when zipping an APK
pub const ZIP_COMPRESSION_METHOD: CompressionMethod = CompressionMethod::Deflated;

/// Represents an APK that has been loaded.
pub struct Apk {
    /// Temporary directory where the APK's unzipped contents are stored.
    pub temp_dir: TempDir,
}

impl Apk {
    /// Load an APK from a path.
    pub fn from_path(apk_path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = File::open(apk_path.as_ref())?;
        let mut zip = ZipArchive::new(file)?;
        let temp_dir = TempDir::new()?;

        zip.extract(temp_dir.path())?;

        Ok(Self { temp_dir })
    }

    /// Zips the APK to `out_path`, compressing it with [`crate::apk::ZIP_COMPRESSION_METHOD`].
    ///
    /// Does not compress the `resources.arsc` file.
    pub fn zip(&self, out_path: impl AsRef<Path>) -> Result<(), Error> {
        let out_file = File::create(out_path)?;

        let mut archive = ZipWriter::new(out_file);
        let compress_options = SimpleFileOptions::default()
            .compression_method(ZIP_COMPRESSION_METHOD)
            .unix_permissions(0o755);
        let no_compress_options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o755);

        let in_path = self.temp_dir.path();
        let mut entry_file_buffer = Vec::new();
        for entry_result in WalkDir::new(in_path) {
            let entry = entry_result?;

            let entry_path = entry.path();
            let entry_relative_path = entry_path.strip_prefix(in_path)?;

            if entry_path.is_file() {
                if entry_path.ends_with("resources.arsc") {
                    archive.start_file_from_path(entry_relative_path, no_compress_options)?;
                } else {
                    archive.start_file_from_path(entry_relative_path, compress_options)?;
                }

                let mut entry_file = File::open(entry_path)?;
                entry_file.read_to_end(&mut entry_file_buffer)?;
                archive.write_all(&entry_file_buffer)?;
                entry_file_buffer.clear();
            } else if !entry_relative_path.as_os_str().is_empty() {
                archive.add_directory_from_path(entry_relative_path, compress_options)?;
            }
        }
        archive.finish()?;

        Ok(())
    }
}
