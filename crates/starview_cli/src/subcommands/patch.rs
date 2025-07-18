use std::{
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    time::Instant,
};

use clap::Parser;
use starview_patch::{
    apk::{self, aligner::ZipAligner, signer::ApkSigner, Apk}, ffdec::{self, FFDec}, replace::Replacements, ScriptPatcher
};

use crate::{Error, color, progress::ProgressBar};

/// Where extracted FFDec files will be placed
const EXTRACT_DIR: &str = "extracted";
const ZIP_FILE_NAME: &str = "apk.zip";
const DEFAULT_OUT_FILE_NAME: &str = "patched.apk";
const DEFAULT_KEYSTORE_PATH: &str = "wf.keystore";
const DEFAULT_KEYSTORE_PASS: &str = "pass:worldflipper";
const DEFAULT_PATCH_PATH: &str = "patches";
const ZIP_ALIGN_BYTES: usize = 4;

#[derive(Parser, Debug)]
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
    pub patch: Vec<String>,

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
    let patch_start_instant = Instant::now();

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

    // load zipaligner
    let zip_aligner = if let Some(aligner_path) = args.zip_align {
        ZipAligner::from_path(aligner_path)
    } else {
        ZipAligner::new()
    }?;

    // load APK
    let apk = load_apk(args.apk_path)?;
    let apk_dir_path = apk.temp_dir.path();

    // load script patcher
    let mut patch_dirs = args.patch;
    if patch_dirs.is_empty() {
        patch_dirs.push(DEFAULT_PATCH_PATH.to_string());
    }
    let patcher = ScriptPatcher::new(
        patch_dirs,
        replacements,
    )?;

    // extract scripts
    let apk_swf_path =
        apk_dir_path.join(args.swf.unwrap_or(apk::DEFAULT_WF_SWF_LOCATION.to_string()));
    let script_extract_path = apk_dir_path.join(EXTRACT_DIR);
    extract_scripts(&ffdec, &apk_swf_path, &script_extract_path, &patcher)?;

    // patch scripts
    patch_scripts(
        &patcher,
        script_extract_path.join(ffdec::FFDEC_SCRIPTS_EXTRACT_DIR),
    )?;

    // import scripts
    import_scripts(&ffdec, &apk_swf_path, &script_extract_path)?;

    // remove extracted scripts directory
    remove_dir_all(script_extract_path)?;

    // zip apk
    let zip_path = apk_dir_path.join(ZIP_FILE_NAME);
    zip_apk(&apk, &zip_path)?;

    // zipalign apk
    align_apk(zip_aligner, ZIP_ALIGN_BYTES, &zip_path, &out_path)?;

    // sign apk
    sign_apk(
        apk_signer,
        out_path,
        PathBuf::from(DEFAULT_KEYSTORE_PATH),
        DEFAULT_KEYSTORE_PASS,
    )?;

    println!(
        "{}Successfully patched apk in {:?}.{}",
        color::SUCCESS.render_fg(),
        Instant::now().duration_since(patch_start_instant),
        color::TEXT.render_fg()
    );

    Ok(())
}

fn load_apk(apk_path: String) -> Result<Apk, Error> {
    println!(
        "{}[1/7] {}Unzipping APK...",
        color::TEXT_VARIANT.render_fg(),
        color::TEXT.render_fg()
    );
    let progress_bar = ProgressBar::spinner();
    let apk = Apk::from_path(apk_path)?;
    progress_bar.finish_and_clear();

    Ok(apk)
}

fn extract_scripts(
    ffdec: &FFDec,
    apk_swf_path: &PathBuf,
    script_extract_path: &PathBuf,
    patcher: &ScriptPatcher,
) -> Result<(), Error> {
    println!(
        "{}[2/7] {}Extracting scripts...",
        color::TEXT_VARIANT.render_fg(),
        color::TEXT.render_fg()
    );
    let progress_bar = ProgressBar::spinner();
    ffdec.extract_scripts(
        apk_swf_path,
        script_extract_path,
        &patcher.get_patch_script_names(),
    )?;
    progress_bar.finish_and_clear();

    Ok(())
}

fn patch_scripts(patcher: &ScriptPatcher, to_patch_dir: PathBuf) -> Result<(), Error> {
    println!(
        "{}[3/7] {}Patching scripts...",
        color::TEXT_VARIANT.render_fg(),
        color::TEXT.render_fg()
    );
    let progress_bar = ProgressBar::spinner();
    patcher.patch(to_patch_dir)?;
    progress_bar.finish_and_clear();

    Ok(())
}

fn import_scripts(
    ffdec: &FFDec,
    apk_swf_path: &PathBuf,
    script_extract_path: &PathBuf,
) -> Result<(), Error> {
    println!(
        "{}[4/7] {}Importing patched scripts...",
        color::TEXT_VARIANT.render_fg(),
        color::TEXT.render_fg()
    );
    let progress_bar = ProgressBar::spinner();
    ffdec.import_scripts(apk_swf_path, script_extract_path)?;
    progress_bar.finish_and_clear();

    Ok(())
}

fn zip_apk(apk: &Apk, out_path: &PathBuf) -> Result<(), Error> {
    println!(
        "{}[5/7] {}Zipping APK...",
        color::TEXT_VARIANT.render_fg(),
        color::TEXT.render_fg()
    );
    let progress_bar = ProgressBar::spinner();
    apk.zip(out_path)?;
    progress_bar.finish_and_clear();

    Ok(())
}

fn align_apk(zip_aligner: ZipAligner, align: usize, in_path: &PathBuf, out_path: &PathBuf) -> Result<(), Error> {
    println!(
        "{}[6/7] {}Zip Aligning APK...",
        color::TEXT_VARIANT.render_fg(),
        color::TEXT.render_fg()
    );
    let progress_bar = ProgressBar::spinner();
    zip_aligner.align(align, in_path, out_path)?;
    progress_bar.finish_and_clear();

    Ok(())
}

fn sign_apk(
    apk_signer: ApkSigner,
    apk_path: PathBuf,
    keystore_path: PathBuf,
    keystore_pass: &str,
) -> Result<(), Error> {
    println!(
        "{}[7/7] {}Signing APK...",
        color::TEXT_VARIANT.render_fg(),
        color::TEXT.render_fg()
    );
    let progress_bar = ProgressBar::spinner();
    apk_signer.sign(apk_path, keystore_path, keystore_pass)?;
    progress_bar.finish_and_clear();

    Ok(())
}
