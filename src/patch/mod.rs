use std::collections::HashMap;

use patch::{Hunk, Line};

use crate::error::Error;

pub mod script;

/// Attempts to apply the provided [`patch::Patch`] to a string.
pub fn apply(old: &str, patch: patch::Patch) -> Result<String, Error> {
    // build hunk map
    let mut hunk_map: HashMap<usize, Hunk> = HashMap::with_capacity(patch.hunks.len());
    for hunk in patch.hunks {
        hunk_map.insert(hunk.old_range.start.try_into()?, hunk);
    }

    let mut lines = old.lines().enumerate();
    let mut new_lines: Vec<&str> = Vec::new();

    while let Some((line_n, line)) = lines.next() {
        if let Some(hunk) = hunk_map.get(&(line_n + 1)) {
            let mut add_lines = 0;
            for hunk_line in hunk.lines.iter() {
                match hunk_line {
                    Line::Add(new_line) => {
                        new_lines.push(&new_line);
                        add_lines += 1;
                    }
                    Line::Remove(_) => {}
                    Line::Context(context_line) => {
                        new_lines.push(&context_line);
                    }
                }
            }

            let old_range: usize = hunk.old_range.count.try_into()?;
            if old_range > add_lines {     
                lines.nth(old_range - add_lines);
            }
        } else {
            new_lines.push(line);
        }
    }

    Ok(new_lines.join("\n"))
}
