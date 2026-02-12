//! Iron FS - Filesystem operations
//!
//! Handles:
//! - Symlink management for dotfiles
//! - Backup creation
//! - Directory traversal
//! - Config file discovery

use std::path::{Path, PathBuf};
use anyhow::Result;

/// Create a symlink from source to target
pub fn create_symlink(source: &Path, target: &Path) -> Result<()> {
    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Backup existing file if present
    if target.exists() && !target.is_symlink() {
        let backup = target.with_extension("iron-backup");
        std::fs::rename(target, &backup)?;
    }

    // Remove existing symlink
    if target.is_symlink() {
        std::fs::remove_file(target)?;
    }

    // Create symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(source, target)?;

    Ok(())
}

/// Remove a symlink
pub fn remove_symlink(target: &Path) -> Result<()> {
    if target.is_symlink() {
        std::fs::remove_file(target)?;
    }
    Ok(())
}

/// Check if path is a symlink pointing to expected source
pub fn is_valid_symlink(target: &Path, expected_source: &Path) -> bool {
    if let Ok(actual_source) = std::fs::read_link(target) {
        actual_source == expected_source
    } else {
        false
    }
}

/// Expand ~ to home directory
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}
