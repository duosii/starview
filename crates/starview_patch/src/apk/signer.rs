use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crate::{error::Error, utils::validate_file_path};

const SIGNER_LOCATIONS: [&str; 1] = [
    "build-tools/apksigner.bat", // Windows; in the same directory as starview in a folder called "build-tools"
];
const SIGNER_FILENAME: &str = "apksigner";

/// Handles signing APKs
pub struct ApkSigner {
    location: PathBuf,
}

impl ApkSigner {
    /// Create a new APKSigner.
    ///
    /// Will try to determine the apksigner binary's install location automatically.
    pub fn new() -> Result<Self, Error> {
        for location in SIGNER_LOCATIONS {
            if let Ok(signer) = Self::from_path(location) {
                return Ok(signer);
            }
        }
        Err(Error::ApkSignerPath())
    }

    /// Create a new APKSigner with a path to the apksigner binary.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();

        match validate_file_path(path, SIGNER_FILENAME)? {
            true => Ok(Self {
                location: path.to_path_buf(),
            }),
            false => Err(Error::ApkSignerPath()),
        }
    }

    /// Signs the APK at `apk_path` using the keystore at `keystore_path`.
    ///
    /// `keystore_pass` defines the keystore's password.
    pub fn sign<P>(
        &self,
        apk_path: P,
        keystore_path: P,
        keystore_pass: &str,
    ) -> Result<Output, Error>
    where
        P: AsRef<Path>,
    {
        let sign_result = Command::new(&self.location)
            .args([
                "sign",
                "--ks",
                &keystore_path.as_ref().to_string_lossy(),
                "--ks-pass",
                keystore_pass,
                &apk_path.as_ref().to_string_lossy(),
            ])
            .output();

        sign_result.map_err(|err| Error::Sign(err.to_string()))
    }
}
