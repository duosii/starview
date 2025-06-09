use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crate::{error::Error, utils::validate_file_path};

const FFDEC_LOCATIONS: [&str; 1] = [
    "ffdec/ffdec.bat", // Windows; in the same directory as starview in a folder called "ffdec"
];
const FFDEC_FILENAME: &str = "ffdec";
const IGNORE_ERROR: &str = "Duplicate pack path found";

/// the name of the directory where FFDEC extracts scripts to.
pub const FFDEC_SCRIPTS_EXTRACT_DIR: &str = "scripts";

pub struct FFDec {
    location: PathBuf,
}

impl FFDec {
    /// Creates a new FFDec interface.
    /// Attempts to find the FFDec install locaton automatically.
    pub fn new() -> Result<Self, Error> {
        for location in FFDEC_LOCATIONS {
            if let Ok(interface) = Self::from_path(location) {
                return Ok(interface);
            }
        }
        Err(Error::FFDecPath())
    }

    /// Creates a new FFDec interface, where the FFDec CLI tool is located at the
    /// provided path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();

        match validate_file_path(path, FFDEC_FILENAME)? {
            true => Ok(Self {
                location: path.to_path_buf(),
            }),
            false => Err(Error::FFDecPath()),
        }
    }

    /// Extracts the scripts from the swf at the provided path.
    /// The extracted scripts will be placed in out_path.
    pub fn extract_scripts<P: AsRef<Path>>(
        &self,
        swf_path: &P,
        out_path: &P,
        class_names: &[String],
    ) -> Result<Output, Error> {
        let extract_result = Command::new(&self.location)
            .args([
                "-selectclass",
                &class_names.join(","),
                "-export",
                "script",
                &out_path.as_ref().to_string_lossy(),
                &swf_path.as_ref().to_string_lossy(),
            ])
            .output();

        extract_result.map_err(|extract_err| Error::FFDecExtract(extract_err.to_string()))
    }

    /// Imports the scripts from the provided path `scripts_path`
    ///
    /// The scripts will be imported into the provided `in_swf_path`
    ///
    /// The .swf file will be modified in-place.
    pub fn import_scripts<P: AsRef<Path>>(
        &self,
        in_swf_path: &P,
        scripts_path: &P,
    ) -> Result<Output, Error> {
        let in_swf_path = in_swf_path.as_ref().to_string_lossy();

        let import_output = Command::new(&self.location)
            .args([
                "-air",
                "-importScript",
                &in_swf_path,
                &in_swf_path,
                &scripts_path.as_ref().to_string_lossy(),
            ])
            .output()
            .map_err(|import_err| Error::FFDecImport(import_err.to_string()))?;

        let errs: Vec<String> = String::from_utf8_lossy(&import_output.stderr)
            .lines()
            .filter_map(|err| {
                (err.contains("SEVERE") && !err.contains(IGNORE_ERROR)).then(|| err.to_string())
            })
            .collect();

        if !errs.is_empty() {
            return Err(Error::FFDecImport(errs.join("\n")));
        }

        Ok(import_output)
    }
}
