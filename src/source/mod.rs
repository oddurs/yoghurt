//! Where facts come from.
//!
//! One module per package manager, each implementing [`crate::model::fact::Source`]
//! and knowing nothing about the others, the graph, or the interface. Adding a
//! package manager is a file here and nothing else.
//!
//! [`walk`] is the exception and is deliberately not an adapter for anything:
//! it reads the filesystem and says what is actually there, so that a claim
//! with no artifact is broken and an artifact with no claim is an orphan.

pub mod cargo;
pub mod homebrew;
pub mod walk;

use std::fs;
use std::path::{Path, PathBuf};

/// Every directory entry that is not a dotfile.
pub(crate) fn children(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .collect();
    found.sort();
    found
}

/// Bytes under a directory, following nothing.
///
/// A keg is a few hundred files, so this is a plain recursive walk rather than
/// anything clever. Symlinks are counted as their own size, never followed —
/// a keg that links into another keg must not be charged for it twice.
pub(crate) fn size_of(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.is_dir() {
        children(path).iter().map(|child| size_of(child)).sum()
    } else {
        metadata.len()
    }
}
