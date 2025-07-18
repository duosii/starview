use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crate::{Error, utils::validate_file_path};

const ALIGNER_LOCATIONS: [&str; 1] = [
    "build-tools/zipalign.exe", // Windows; in the same directory as starview in a folder called "build-tools"
];
const ALIGNER_FILENAME: &str = "zipalign";

/// Aligns zip files at a byte-level
pub struct ZipAligner {
    location: PathBuf,
}

impl ZipAligner {
    /// Create a new ZipAligner.
    ///
    /// Attempts to automatically determine
    /// the location of the ZipAligner executable.
    ///
    /// If not, this function will return an Error::ZipAlignerPath
    pub fn new() -> Result<Self, Error> {
        for location in ALIGNER_LOCATIONS {
            if let Ok(aligner) = Self::from_path(location) {
                return Ok(aligner);
            }
        }
        Err(Error::ZipAlignerPath)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();

        if validate_file_path(path, ALIGNER_FILENAME)? {
            Ok(Self {
                location: path.to_path_buf(),
            })
        } else {
            Err(Error::ZipAlignerPath)
        }
    }

    /// Aligns the archive located at `in_path`, outputting to `out_path`.
    ///
    /// `align` determines the
    pub fn align<P>(&self, align: usize, in_path: P, out_path: P) -> Result<Output, Error>
    where
        P: AsRef<Path>,
    {
        let in_path = in_path.as_ref().to_str().ok_or(Error::ZipAlign(
            "could not convert in_path to a string".into(),
        ))?;

        let out_path = out_path.as_ref().to_str().ok_or(Error::ZipAlign(
            "could not convert in_path to a string".into(),
        ))?;

        let sign_result = Command::new(&self.location)
            .args([
                align.to_string(),
                in_path.to_string(),
                out_path.to_string(),
            ])
            .output();
        
        sign_result.map_err(|err| Error::ZipAlign(err.to_string()))
    }
}
