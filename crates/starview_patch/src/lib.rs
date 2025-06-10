mod error;
mod script;
mod utils;

pub mod apk;
pub mod ffdec;

use patch::{Hunk, Line};
use std::collections::HashMap;

pub use error::Error;
pub use script::ScriptPatcher;

/// Attempts to apply the provided [`patch::Patch`] to a string.
fn apply_patch(old: &str, patch: patch::Patch) -> Result<String, Error> {
    // build hunk map
    let mut hunk_map: HashMap<usize, Hunk> = HashMap::with_capacity(patch.hunks.len());
    for hunk in patch.hunks {
        hunk_map.insert(hunk.old_range.start.try_into()?, hunk);
    }

    let mut lines = old.lines().enumerate();
    let mut new_lines: Vec<&str> = Vec::new();

    while let Some((line_n, line)) = lines.next() {
        if let Some(hunk) = hunk_map.get(&(line_n + 1)) {
            // insert the current line if the patch is only adding
            if hunk.old_range.count == 0 {
                new_lines.push(line);
            }
            for (hunk_line_n, hunk_line) in hunk.lines.iter().enumerate() {
                match hunk_line {
                    Line::Add(new_line) => {
                        new_lines.push(&new_line);
                    }
                    Line::Remove(_) => {
                        if hunk_line_n != 0 {
                            lines.next();
                        }
                    }
                    Line::Context(context_line) => {
                        new_lines.push(&context_line);
                        if hunk_line_n != 0 {
                            lines.next();
                        }
                    }
                }
            }
        } else {
            new_lines.push(line);
        }
    }

    Ok(new_lines.join("\n"))
}
