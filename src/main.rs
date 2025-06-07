mod apk;
mod error;
mod ffdec;
mod patch;
mod utils;

use ::patch::Patch;
use apk::{Apk, signer::ApkSigner};
use clap::Parser;
use error::Error;
use ffdec::FFDec;
use patch::script::ScriptPatcher;

use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};

use crate::patch::apply;

/// Where extracted FFDec files will be placed
const EXTRACT_DIR: &str = "extracted";
const DEFAULT_OUT_FILE_NAME: &str = "patched.apk";

const DEFAULT_KEYSTORE_PATH: &str = "wf.keystore";
const DEFAULT_KEYSTORE_PASS: &str = "pass:worldflipper";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
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

    /// Path to the APK file
    pub apk_path: String,

    /// Where the patched APK file will be written to.
    pub out_path: String,
}

/// Patches the apk at the provided path.
fn patch_apk<P>(
    apk_path: P,
    out_path: P,
    ffdec_path: Option<String>,
    signer_path: Option<String>,
    swf_path: Option<String>,
) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    // if out_path is a directory, append the default file name to the path
    let out_path = {
        let path = out_path.as_ref();
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

    // load ffdec interface
    let ffdec = if let Some(custom_ffdec_path) = ffdec_path {
        FFDec::from_path(custom_ffdec_path)
    } else {
        FFDec::new()
    }?;

    // load apksigner
    let apk_signer = if let Some(signer_path) = signer_path {
        ApkSigner::from_path(signer_path)
    } else {
        ApkSigner::new()
    }?;

    // load APK
    println!("unzipping...");
    let apk = Apk::from_path(apk_path)?;
    let apk_dir_path = apk.temp_dir.path();
    println!("  unzipped to: {:?}", apk_dir_path);

    // load script patcher
    let patcher = ScriptPatcher::new("patches").unwrap();

    // extract scripts
    println!("extracting scripts...");
    let apk_swf_path =
        apk_dir_path.join(swf_path.unwrap_or(apk::DEFAULT_WF_SWF_LOCATION.to_string()));
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

fn main() {
    let args_parse_result = Args::try_parse();
    let patch_result = match args_parse_result {
        Ok(args) => patch_apk(
            args.apk_path,
            args.out_path,
            args.ffdec,
            args.sign,
            args.swf,
        ),
        Err(parse_err) => parse_err.print().map_err(|err| err.into()),
    };

    if let Err(err) = patch_result {
        println!("error: {}", err);
    }
}
