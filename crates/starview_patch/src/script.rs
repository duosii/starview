use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{apply_patch, error::Error, replace::Replacements};

/// Handles patching ActionScript files from a directory of diff patches
pub struct ScriptPatcher {
    /// The paths of all of the patches that this ScriptPatcher will apply
    patch_paths: Vec<PathBuf>,

    /// Replacements for patches
    replacements: Option<Replacements>,
}

impl ScriptPatcher {
    /// Create a new ScriptPatcher that will load patches from the provided directory
    pub fn new(
        patches_path: impl AsRef<Path>,
        replacements: Option<Replacements>,
    ) -> Result<Self, Error> {
        let patches_path = patches_path.as_ref();

        if !patches_path.is_dir() {
            return Err(Error::NotDirectory(
                patches_path.to_string_lossy().to_string(),
            ));
        }

        let paths = WalkDir::new(&patches_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .map(|patch_file_entry| {
                let patch_file_entry = patch_file_entry?;
                let patch_file_path = patch_file_entry.path();
                Ok(patch_file_path.to_path_buf())
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(Self {
            patch_paths: paths,
            replacements,
        })
    }

    /// Pulling from this ScriptPatcher's patches, patches all matching
    /// scripts in the provided directory.
    pub fn patch(&self, to_patch_dir: impl AsRef<Path>) -> Result<(), Error> {
        let to_patch_dir = to_patch_dir.as_ref();
        for patch_file_path in &self.patch_paths {
            if patch_file_path.is_file() {
                // load the patch file & parse it into a [`diffy::Patch``]
                let patch_file_string = read_file_to_string(patch_file_path)?;

                // replace patch text if any replacements exist
                let patch_file_string = if let Some(replacements) = &self.replacements {
                    replacements.replace(&patch_file_string)
                } else {
                    patch_file_string
                };

                let patch = patch::Patch::from_single(&patch_file_string)
                    .map_err(|err| Error::PatchParse(err.to_string()))?;

                let modified_file_name = patch.new.path.to_string();

                // load the file that we will patch
                let to_patch_file_path = to_patch_dir.join(&modified_file_name);
                if !to_patch_file_path.try_exists()? {
                    return Err(Error::ToPatchFileMissing(modified_file_name));
                }

                let mut to_patch_file = File::options()
                    .read(true)
                    .write(true)
                    .open(to_patch_file_path)?;

                let mut to_patch_file_string = String::new();
                to_patch_file.read_to_string(&mut to_patch_file_string)?;

                // apply patch & write patched string to file
                to_patch_file_string = apply_patch(&to_patch_file_string, patch)?;
                to_patch_file.set_len(0)?;
                to_patch_file.seek(std::io::SeekFrom::Start(0))?;
                to_patch_file.write_all(to_patch_file_string.as_bytes())?;
            }
        }

        Ok(())
    }

    /// Returns the file stems of all of this patcher's patches
    pub fn get_patch_script_names(&self) -> Vec<String> {
        self.patch_paths
            .iter()
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| Some(stem.to_string_lossy().to_string()))
            })
            .collect()
    }
}

/// Reads a file to a string
fn read_file_to_string<'a>(path: &PathBuf) -> Result<String, std::io::Error> {
    let mut patch_file = File::open(path)?;
    let mut patch_file_string = String::new();

    patch_file.read_to_string(&mut patch_file_string)?;

    Ok(patch_file_string)
}
