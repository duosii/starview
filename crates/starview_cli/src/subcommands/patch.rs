use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

use clap::{Parser, command};
use starview_patch::{
    ScriptPatcher,
    apk::{self, Apk, signer::ApkSigner},
    ffdec::{self, FFDec},
    replace::Replacements,
};

use crate::Error;

/// Where extracted FFDec files will be placed
const EXTRACT_DIR: &str = "extracted";
const DEFAULT_OUT_FILE_NAME: &str = "patched.apk";
const DEFAULT_KEYSTORE_PATH: &str = "wf.keystore";
const DEFAULT_KEYSTORE_PASS: &str = "pass:worldflipper";
const DEFAULT_PATCH_PATH: &str = "patches";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The location of the FFDec program
    #[arg(long, short)]
    pub ffdec: Option<String>,

    /// The location of the APK signer binary
    #[arg(long, short)]
    pub sign: Option<String>,

    /// The location of the keystore that will be used to sign the APK
    #[arg(long, short)]
    pub keystore: Option<String>,

    /// The keystore's password
    #[arg(long)]
    pub ks_pass: Option<String>,

    /// The location of the zip aligner binary
    #[arg(long, short)]
    pub zip_align: Option<String>,

    /// The location of the .swf file inside the APK
    /// By default, this is `assets/worldflipper_android_release.swf`
    #[arg(long)]
    pub swf: Option<String>,

    /// The location of the patches
    /// By default, this is `patches`
    #[arg(long, short)]
    pub patch: Option<String>,

    /// Strings to replace in patches
    /// In the format to_replace=replace_with
    #[arg(long, short)]
    pub replace: Option<String>,

    /// Path to the APK file
    pub apk_path: String,

    /// Where the patched APK file will be written to.
    pub out_path: String,
}

pub fn patch(args: Args) -> Result<(), Error> {
    // if out_path is a directory, append the default file name to the path
    let out_path = {
        let path = PathBuf::from(args.out_path);
        let new_path = if path.is_dir() {
            path.join(DEFAULT_OUT_FILE_NAME)
        } else {
            path.to_path_buf()
        };

        if let Some(parent) = new_path.parent() {
            if parent.is_dir() && !parent.try_exists()? {
                create_dir_all(path)?;
            }
        }
        new_path
    };

    let replacements = if let Some(replace_str) = args.replace {
        Some(Replacements::try_parse_str(&replace_str)?)
    } else {
        None
    };

    // load ffdec interface
    let ffdec = if let Some(custom_ffdec_path) = args.ffdec {
        FFDec::from_path(custom_ffdec_path)
    } else {
        FFDec::new()
    }?;

    // load apksigner
    let apk_signer = if let Some(signer_path) = args.sign {
        ApkSigner::from_path(signer_path)
    } else {
        ApkSigner::new()
    }?;

    // load APK
    println!("unzipping...");
    let apk = Apk::from_path(args.apk_path)?;
    let apk_dir_path = apk.temp_dir.path();
    println!("  unzipped to: {:?}", apk_dir_path);

    // load script patcher
    let patcher = ScriptPatcher::new(
        args.patch.unwrap_or(DEFAULT_PATCH_PATH.to_string()),
        replacements,
    )?;

    // extract scripts
    println!("extracting scripts...");
    let apk_swf_path =
        apk_dir_path.join(args.swf.unwrap_or(apk::DEFAULT_WF_SWF_LOCATION.to_string()));
    let script_extract_path = apk_dir_path.join(EXTRACT_DIR);
    ffdec.extract_scripts(
        &apk_swf_path,
        &script_extract_path,
        &patcher.get_patch_script_names(),
    )?;
    println!("  extracted to: {:?}", &script_extract_path);

    // patch scripts
    println!("patching scripts...");
    patcher.patch(script_extract_path.join(ffdec::FFDEC_SCRIPTS_EXTRACT_DIR))?;
    println!("  patched scripts");

    // import scripts
    println!("importing scripts...");
    ffdec.import_scripts(&apk_swf_path, &script_extract_path)?;

    // remove extracted scripts directory
    println!("removing extracted scripts...");
    remove_dir_all(script_extract_path)?;

    // zip apk
    println!("zipping patched APK...");
    apk.zip(&out_path)?;

    // zipalign apk
    println!("zipaligning APK...");

    // sign apk
    println!("signing APK...");
    apk_signer.sign(
        out_path,
        PathBuf::from(DEFAULT_KEYSTORE_PATH),
        DEFAULT_KEYSTORE_PASS,
    )?;

    Ok(())
}
