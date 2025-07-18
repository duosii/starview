# starview
A command-line tool for patching the CN version of a pinball game.

starview can additionally download the game's assets while the servers are still live.

You can download the most recent version on the [releases page](https://github.com/duosii/starview/releases/latest).

## Usage
Use the --help flag when running any command to see detailed usage information.

### Patching an APK
```bash
# Patch the APK to send requests to http://localhost:3000
# and bypass ID verification.
starview patch --replace "api_scheme=http,api_host=localhost:3000" <path_to_original_apk> patched.apk
```

### Downloading Assets
```bash
# Download the game's assets ~10GB
starview fetch assets <out_path>

# Download the game's asset path file
starview fetch path <out_path>

# Download the game's asset entity lists
starview fetch list <out_path>
```

## Creating Patches
The [patches that are applied using starview](./patches) are `git diffs`.

If you have git installed, you can generate one with the following command:
```bash
git diff -p -U0 <original_file> <modified_file> > <package path>.patch
```

At the top of the newly generated patch, you will see something like:
```patch
--- a/<original_file_name>.as
+++ b/<modified_file_name>.as
```
Replace `a` and `b` with the fully qualified package path of the file you are generating a patch for.
```patch
--- path/to/file/<original_file_name>.as
+++ path/to/file/<original_file_name>.as
```
Both lines should be the same aside from the `---` and `+++`

See the [patches directory](./patches) for examples of how patches should be formatted.

## Building
### Dependencies
- Install [Rust](https://www.rust-lang.org/tools/install) for your platform and ensure that it's up-to-date.
  ```
  rustup update
  ```
- Install [FFDec](https://github.com/jindrapetrik/jpexs-decompiler/releases/tag/version24.0.1) for your platform.
- Install [Android SDK Build-Tools](https://androidsdkmanager.azurewebsites.net/build_tools.html) for your platform.

To build for debugging:
```
cargo run
```

To build for release:
```
cargo run --release
```

Once built, place `FFDec` and the  `Android SDK Build-Tools` in the same location as the starview binary in folders named `ffdec` and `build-tools`.
For reference, you can view the windows release on the [releases page](https://github.com/duosii/starview/releases/latest)

### Acknowledgements
- [jindrapetrik](https://github.com/jindrapetrik) for creating [FFDec](https://github.com/jindrapetrik/jpexs-decompiler), which makes patching the APK's scripts possible.