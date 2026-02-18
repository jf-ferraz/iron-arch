//! Iron FS - Filesystem operations for Iron
//!
//! This crate provides filesystem abstractions for Iron:
//! - TOML configuration parsing
//! - Symlink management for dotfiles
//! - Backup creation with timestamps
//! - Atomic file operations
//! - Directory traversal utilities
//! - Path expansion (home dir, env vars)

use chrono::Local;
use iron_core::{FsError, IronResult};
use serde::de::DeserializeOwned;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Backup file extension
const BACKUP_EXTENSION: &str = "iron-backup";

/// TOML configuration parser
pub mod config {
    use super::*;
    use iron_core::ConfigError;

    /// Parse a TOML file into a type
    pub fn parse_toml<T: DeserializeOwned>(path: &Path) -> IronResult<T> {
        if !path.exists() {
            return Err(ConfigError::NotFound {
                path: path.to_path_buf(),
            }
            .into());
        }

        let content = fs::read_to_string(path).map_err(|e| ConfigError::ParseError {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

        toml::from_str(&content).map_err(|e| {
            ConfigError::ParseError {
                path: path.to_path_buf(),
                message: e.to_string(),
            }
            .into()
        })
    }

    /// Parse TOML content from a string
    pub fn parse_toml_str<T: DeserializeOwned>(content: &str, path: &Path) -> IronResult<T> {
        toml::from_str(content).map_err(|e| {
            ConfigError::ParseError {
                path: path.to_path_buf(),
                message: e.to_string(),
            }
            .into()
        })
    }

    /// Serialize a type to TOML and write to file
    pub fn write_toml<T: serde::Serialize>(path: &Path, value: &T) -> IronResult<()> {
        let content = toml::to_string_pretty(value).map_err(|e| ConfigError::ParseError {
            path: path.to_path_buf(),
            message: format!("Serialization failed: {}", e),
        })?;

        atomic_write(path, content.as_bytes())
    }
}

/// Symlink management
pub mod symlink {
    use super::*;

    /// Symlink status information
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SymlinkStatus {
        /// Valid symlink pointing to expected source
        Valid,
        /// Symlink exists but points elsewhere
        WrongTarget { actual: PathBuf },
        /// Target exists but is not a symlink
        NotSymlink,
        /// Target does not exist
        Missing,
    }

    /// Create a symlink from source to target
    ///
    /// If target exists and is not a symlink, it will be backed up.
    /// If target is a symlink pointing elsewhere, it will be replaced.
    pub fn create(source: &Path, target: &Path) -> IronResult<()> {
        // Validate source exists
        if !source.exists() {
            return Err(FsError::NotFound {
                path: source.to_path_buf(),
            }
            .into());
        }

        // Create parent directories if needed
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| FsError::PermissionDenied {
                path: parent.to_path_buf(),
            })?;
        }

        // Handle existing target
        if target.exists() || target.is_symlink() {
            if target.is_symlink() {
                // Remove existing symlink
                fs::remove_file(target).map_err(|_| FsError::PermissionDenied {
                    path: target.to_path_buf(),
                })?;
            } else {
                // Backup existing file/directory
                super::backup::create(target)?;
                if target.is_dir() {
                    fs::remove_dir_all(target).map_err(|_| FsError::PermissionDenied {
                        path: target.to_path_buf(),
                    })?;
                } else {
                    fs::remove_file(target).map_err(|_| FsError::PermissionDenied {
                        path: target.to_path_buf(),
                    })?;
                }
            }
        }

        // Create symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(source, target).map_err(|_| FsError::PermissionDenied {
            path: target.to_path_buf(),
        })?;

        Ok(())
    }

    /// Remove a symlink and optionally restore backup
    pub fn remove(target: &Path, restore_backup: bool) -> IronResult<()> {
        if !target.is_symlink() {
            if target.exists() {
                return Err(FsError::NotASymlink {
                    path: target.to_path_buf(),
                }
                .into());
            }
            return Ok(()); // Already gone
        }

        fs::remove_file(target).map_err(|_| FsError::PermissionDenied {
            path: target.to_path_buf(),
        })?;

        if restore_backup {
            super::backup::restore_latest(target)?;
        }

        Ok(())
    }

    /// Get the status of a symlink target
    pub fn status(target: &Path, expected_source: &Path) -> SymlinkStatus {
        if !target.exists() && !target.is_symlink() {
            return SymlinkStatus::Missing;
        }

        if !target.is_symlink() {
            return SymlinkStatus::NotSymlink;
        }

        match fs::read_link(target) {
            Ok(actual) => {
                if actual == expected_source {
                    SymlinkStatus::Valid
                } else {
                    SymlinkStatus::WrongTarget { actual }
                }
            }
            Err(_) => SymlinkStatus::Missing,
        }
    }

    /// Check if target is a valid symlink pointing to expected source
    pub fn is_valid(target: &Path, expected_source: &Path) -> bool {
        matches!(status(target, expected_source), SymlinkStatus::Valid)
    }
}

/// Backup management
pub mod backup {
    use super::*;

    /// Create a timestamped backup of a file or directory
    pub fn create(path: &Path) -> IronResult<PathBuf> {
        if !path.exists() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            }
            .into());
        }

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!(
            "{}.{}.{}",
            path.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default(),
            timestamp,
            BACKUP_EXTENSION
        );
        let backup_path = path.with_file_name(backup_name);

        if path.is_dir() {
            copy_dir_recursive(path, &backup_path)?;
        } else {
            fs::copy(path, &backup_path).map_err(|e| FsError::BackupFailed {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;
        }

        Ok(backup_path)
    }

    /// Restore the most recent backup for a path
    pub fn restore_latest(original_path: &Path) -> IronResult<bool> {
        let backups = list_backups(original_path)?;
        if backups.is_empty() {
            return Ok(false);
        }

        // Get the most recent backup (list is sorted by modification time)
        let latest = &backups[0];
        restore(latest, original_path)?;

        Ok(true)
    }

    /// Restore a specific backup to the original path
    pub fn restore(backup_path: &Path, original_path: &Path) -> IronResult<()> {
        if !backup_path.exists() {
            return Err(FsError::NotFound {
                path: backup_path.to_path_buf(),
            }
            .into());
        }

        // Remove existing file/dir at original path
        if original_path.exists() {
            if original_path.is_dir() {
                fs::remove_dir_all(original_path).map_err(|e| FsError::RestoreFailed {
                    path: original_path.to_path_buf(),
                    message: e.to_string(),
                })?;
            } else {
                fs::remove_file(original_path).map_err(|e| FsError::RestoreFailed {
                    path: original_path.to_path_buf(),
                    message: e.to_string(),
                })?;
            }
        }

        if backup_path.is_dir() {
            copy_dir_recursive(backup_path, original_path)?;
        } else {
            fs::copy(backup_path, original_path).map_err(|e| FsError::RestoreFailed {
                path: original_path.to_path_buf(),
                message: e.to_string(),
            })?;
        }

        Ok(())
    }

    /// List all backups for a path (sorted by modification time, newest first)
    pub fn list_backups(original_path: &Path) -> IronResult<Vec<PathBuf>> {
        let parent = original_path.parent().unwrap_or(Path::new("."));
        let file_name = original_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let prefix = format!("{}.", file_name);
        let suffix = format!(".{}", BACKUP_EXTENSION);

        let mut backups: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&prefix)
                    && name.ends_with(&suffix)
                    && let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                {
                    backups.push((entry.path(), modified));
                }
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(backups.into_iter().map(|(path, _)| path).collect())
    }

    /// Clean old backups, keeping only the most recent N
    pub fn cleanup(original_path: &Path, keep_count: usize) -> IronResult<usize> {
        let backups = list_backups(original_path)?;
        let mut removed = 0;

        for backup in backups.iter().skip(keep_count) {
            if backup.is_dir() {
                let _ = fs::remove_dir_all(backup);
            } else {
                let _ = fs::remove_file(backup);
            }
            removed += 1;
        }

        Ok(removed)
    }
}

/// Atomic file operations
pub fn atomic_write(path: &Path, content: &[u8]) -> IronResult<()> {
    // Create parent directories
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| FsError::PermissionDenied {
            path: parent.to_path_buf(),
        })?;
    }

    // Write to temporary file first
    let temp_path = path.with_extension("iron-tmp");
    let mut file = File::create(&temp_path).map_err(|_| FsError::PermissionDenied {
        path: temp_path.clone(),
    })?;

    file.write_all(content).map_err(|e| FsError::BackupFailed {
        path: path.to_path_buf(),
        message: format!("Write failed: {}", e),
    })?;

    file.sync_all().map_err(|e| FsError::BackupFailed {
        path: path.to_path_buf(),
        message: format!("Sync failed: {}", e),
    })?;

    // Atomic rename
    fs::rename(&temp_path, path).map_err(|e| FsError::BackupFailed {
        path: path.to_path_buf(),
        message: format!("Rename failed: {}", e),
    })?;

    Ok(())
}

/// Read file content
pub fn read_file(path: &Path) -> IronResult<Vec<u8>> {
    if !path.exists() {
        return Err(FsError::NotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    let mut file = File::open(path).map_err(|_| FsError::PermissionDenied {
        path: path.to_path_buf(),
    })?;

    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .map_err(|_| FsError::PermissionDenied {
            path: path.to_path_buf(),
        })?;

    Ok(content)
}

/// Read file as string
pub fn read_file_string(path: &Path) -> IronResult<String> {
    let content = read_file(path)?;
    String::from_utf8(content).map_err(|e| iron_core::IronError::OperationFailed {
        message: format!("Invalid UTF-8 in {}: {}", path.display(), e),
    })
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> IronResult<()> {
    fs::create_dir_all(dst).map_err(|e| FsError::BackupFailed {
        path: dst.to_path_buf(),
        message: e.to_string(),
    })?;

    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry.map_err(|e| FsError::BackupFailed {
            path: src.to_path_buf(),
            message: e.to_string(),
        })?;

        let relative = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| FsError::BackupFailed {
                path: target.clone(),
                message: e.to_string(),
            })?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| FsError::BackupFailed {
                path: target.clone(),
                message: e.to_string(),
            })?;
        }
    }

    Ok(())
}

/// Path utilities
pub mod path {
    use super::*;
    use std::env;

    /// Expand ~ and environment variables in path
    pub fn expand(path: &str) -> PathBuf {
        let expanded = expand_home(path);
        expand_env_vars(&expanded)
    }

    /// Expand ~ to home directory
    pub fn expand_home(path: &str) -> String {
        if path.starts_with("~/") {
            if let Ok(home) = env::var("HOME") {
                return format!("{}{}", home, &path[1..]);
            }
        } else if path == "~"
            && let Ok(home) = env::var("HOME")
        {
            return home;
        }
        path.to_string()
    }

    /// Expand environment variables in path (${VAR} or $VAR syntax)
    pub fn expand_env_vars(path: &str) -> PathBuf {
        let mut result = path.to_string();

        // Expand ${VAR} syntax
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let replacement = env::var(var_name).unwrap_or_default();
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    replacement,
                    &result[start + end + 1..]
                );
            } else {
                break;
            }
        }

        // Expand $VAR syntax (word characters only)
        let mut i = 0;
        while i < result.len() {
            if result[i..].starts_with('$') && !result[i..].starts_with("${") {
                let start = i;
                i += 1;
                let mut end = i;
                for c in result[i..].chars() {
                    if c.is_alphanumeric() || c == '_' {
                        end += c.len_utf8();
                    } else {
                        break;
                    }
                }
                if end > start + 1 {
                    let var_name = &result[start + 1..end];
                    let replacement = env::var(var_name).unwrap_or_default();
                    result = format!("{}{}{}", &result[..start], replacement, &result[end..]);
                    i = start + replacement.len();
                    continue;
                }
            }
            i += 1;
        }

        PathBuf::from(result)
    }

    /// Normalize a path (resolve . and ..)
    pub fn normalize(path: &Path) -> PathBuf {
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {}
                _ => components.push(component),
            }
        }

        components.iter().collect()
    }

    /// Check if path is within a root directory (no escaping via ..)
    pub fn is_within(path: &Path, root: &Path) -> bool {
        let normalized = normalize(path);
        let normalized_root = normalize(root);

        normalized.starts_with(&normalized_root)
    }
}

/// Directory traversal utilities
pub mod traverse {
    use super::*;

    /// Options for directory traversal
    #[derive(Debug, Clone, Default)]
    pub struct TraverseOptions {
        /// Maximum depth to traverse (None for unlimited)
        pub max_depth: Option<usize>,
        /// Follow symlinks
        pub follow_symlinks: bool,
        /// Include hidden files (starting with .)
        pub include_hidden: bool,
        /// File extensions to include (empty for all)
        pub extensions: Vec<String>,
    }

    /// Traverse a directory and return all matching files
    pub fn find_files(root: &Path, options: &TraverseOptions) -> IronResult<Vec<PathBuf>> {
        if !root.exists() {
            return Err(FsError::NotFound {
                path: root.to_path_buf(),
            }
            .into());
        }

        let mut walker = WalkDir::new(root);

        if let Some(depth) = options.max_depth {
            walker = walker.max_depth(depth);
        }

        walker = walker.follow_links(options.follow_symlinks);

        let files: Vec<PathBuf> = walker
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                if !options.include_hidden {
                    !e.file_name().to_string_lossy().starts_with('.')
                } else {
                    true
                }
            })
            .filter(|e| {
                if options.extensions.is_empty() {
                    true
                } else {
                    e.path()
                        .extension()
                        .map(|ext| {
                            options
                                .extensions
                                .contains(&ext.to_string_lossy().to_string())
                        })
                        .unwrap_or(false)
                }
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        Ok(files)
    }

    /// Find all TOML config files in a directory
    pub fn find_config_files(root: &Path) -> IronResult<Vec<PathBuf>> {
        find_files(
            root,
            &TraverseOptions {
                extensions: vec!["toml".to_string()],
                include_hidden: false,
                ..Default::default()
            },
        )
    }

    /// Get directory size in bytes
    pub fn directory_size(path: &Path) -> IronResult<u64> {
        if !path.exists() {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            }
            .into());
        }

        let size = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        atomic_write(&file_path, b"hello world").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("sub/dir/test.txt");

        atomic_write(&file_path, b"nested content").unwrap();

        assert!(file_path.exists());
    }

    #[test]
    fn test_symlink_create_and_status() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("target.txt");

        fs::write(&source, "test content").unwrap();

        symlink::create(&source, &target).unwrap();

        assert!(target.is_symlink());
        assert_eq!(
            symlink::status(&target, &source),
            symlink::SymlinkStatus::Valid
        );
    }

    #[test]
    fn test_symlink_wrong_target() {
        let temp_dir = TempDir::new().unwrap();
        let source1 = temp_dir.path().join("source1.txt");
        let source2 = temp_dir.path().join("source2.txt");
        let target = temp_dir.path().join("target.txt");

        fs::write(&source1, "content1").unwrap();
        fs::write(&source2, "content2").unwrap();

        symlink::create(&source1, &target).unwrap();

        // Check status against different source
        let status = symlink::status(&target, &source2);
        assert!(matches!(status, symlink::SymlinkStatus::WrongTarget { .. }));
    }

    #[test]
    fn test_backup_create_and_list() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("file.txt");

        fs::write(&original, "original content").unwrap();

        let backup_path = backup::create(&original).unwrap();
        assert!(backup_path.exists());

        let backups = backup::list_backups(&original).unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0], backup_path);
    }

    #[test]
    fn test_backup_restore() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("file.txt");

        fs::write(&original, "original content").unwrap();
        let backup_path = backup::create(&original).unwrap();

        // Modify original
        fs::write(&original, "modified content").unwrap();

        // Restore from backup
        backup::restore(&backup_path, &original).unwrap();

        let content = fs::read_to_string(&original).unwrap();
        assert_eq!(content, "original content");
    }

    #[test]
    fn test_path_expand_home() {
        // SAFETY: This is a test running in isolation
        unsafe {
            std::env::set_var("HOME", "/home/test");
        }
        assert_eq!(path::expand_home("~/config"), "/home/test/config");
        assert_eq!(path::expand_home("~"), "/home/test");
        assert_eq!(path::expand_home("/absolute/path"), "/absolute/path");
    }

    #[test]
    fn test_path_expand_env_vars() {
        // SAFETY: This is a test running in isolation
        unsafe {
            std::env::set_var("TEST_VAR", "value");
            std::env::set_var("ANOTHER", "other");
        }

        assert_eq!(
            path::expand_env_vars("${TEST_VAR}/path"),
            PathBuf::from("value/path")
        );
        assert_eq!(
            path::expand_env_vars("$TEST_VAR/path"),
            PathBuf::from("value/path")
        );
        assert_eq!(
            path::expand_env_vars("${TEST_VAR}/${ANOTHER}"),
            PathBuf::from("value/other")
        );
    }

    #[test]
    fn test_path_normalize() {
        assert_eq!(
            path::normalize(Path::new("/a/b/../c")),
            PathBuf::from("/a/c")
        );
        assert_eq!(
            path::normalize(Path::new("/a/./b/c")),
            PathBuf::from("/a/b/c")
        );
    }

    #[test]
    fn test_path_is_within() {
        let root = Path::new("/home/user");
        assert!(path::is_within(Path::new("/home/user/file"), root));
        assert!(path::is_within(Path::new("/home/user/sub/file"), root));
        assert!(!path::is_within(Path::new("/home/other/file"), root));
        assert!(!path::is_within(Path::new("/home/user/../other"), root));
    }

    #[test]
    fn test_traverse_find_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test structure
        fs::write(temp_dir.path().join("a.txt"), "").unwrap();
        fs::write(temp_dir.path().join("b.toml"), "").unwrap();
        fs::create_dir(temp_dir.path().join("sub")).unwrap();
        fs::write(temp_dir.path().join("sub/c.txt"), "").unwrap();

        let all_files =
            traverse::find_files(temp_dir.path(), &traverse::TraverseOptions::default()).unwrap();
        assert_eq!(all_files.len(), 3);

        let toml_files = traverse::find_files(
            temp_dir.path(),
            &traverse::TraverseOptions {
                extensions: vec!["toml".to_string()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(toml_files.len(), 1);
    }

    #[test]
    fn test_config_parse_toml() {
        use serde::Deserialize;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
            id = "test-module"
            name = "Test Module"
            enabled = true
            "#,
        )
        .unwrap();

        #[derive(Deserialize)]
        struct TestConfig {
            id: String,
            name: String,
            enabled: bool,
        }

        let config: TestConfig = config::parse_toml(&config_path).unwrap();
        assert_eq!(config.id, "test-module");
        assert_eq!(config.name, "Test Module");
        assert!(config.enabled);
    }

    // ==========================================================================
    // SymlinkStatus Tests
    // ==========================================================================

    #[test]
    fn test_symlink_status_missing() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("nonexistent.txt");

        fs::write(&source, "content").unwrap();

        let status = symlink::status(&target, &source);
        assert_eq!(status, symlink::SymlinkStatus::Missing);
    }

    #[test]
    fn test_symlink_status_not_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("regular_file.txt");

        fs::write(&source, "content").unwrap();
        fs::write(&target, "regular content").unwrap();

        let status = symlink::status(&target, &source);
        assert_eq!(status, symlink::SymlinkStatus::NotSymlink);
    }

    #[test]
    fn test_symlink_status_debug() {
        let statuses = vec![
            symlink::SymlinkStatus::Valid,
            symlink::SymlinkStatus::Missing,
            symlink::SymlinkStatus::NotSymlink,
            symlink::SymlinkStatus::WrongTarget {
                actual: PathBuf::from("/test"),
            },
        ];

        for status in statuses {
            let debug_str = format!("{:?}", status);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_symlink_status_clone() {
        let status = symlink::SymlinkStatus::WrongTarget {
            actual: PathBuf::from("/path"),
        };
        let cloned = status.clone();
        assert_eq!(cloned, status);
    }

    // ==========================================================================
    // Symlink Edge Cases
    // ==========================================================================

    #[test]
    fn test_symlink_create_fails_for_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("nonexistent.txt");
        let target = temp_dir.path().join("target.txt");

        let result = symlink::create(&source, &target);
        assert!(result.is_err());
    }

    #[test]
    fn test_symlink_create_replaces_existing_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let source1 = temp_dir.path().join("source1.txt");
        let source2 = temp_dir.path().join("source2.txt");
        let target = temp_dir.path().join("target.txt");

        fs::write(&source1, "content1").unwrap();
        fs::write(&source2, "content2").unwrap();

        // Create first symlink
        symlink::create(&source1, &target).unwrap();
        assert!(target.is_symlink());

        // Replace with new symlink
        symlink::create(&source2, &target).unwrap();
        assert!(target.is_symlink());
        assert_eq!(
            symlink::status(&target, &source2),
            symlink::SymlinkStatus::Valid
        );
    }

    #[test]
    fn test_symlink_create_backs_up_regular_file() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("target.txt");

        fs::write(&source, "source content").unwrap();
        fs::write(&target, "original target content").unwrap();

        symlink::create(&source, &target).unwrap();

        assert!(target.is_symlink());
        // A backup should have been created
        let backups = backup::list_backups(&target).unwrap();
        assert_eq!(backups.len(), 1);
    }

    // ==========================================================================
    // Config Module Tests
    // ==========================================================================

    #[test]
    fn test_config_parse_toml_not_found() {
        use serde::Deserialize;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");

        #[derive(Deserialize)]
        struct DummyConfig {
            key: String,
        }

        let result: Result<DummyConfig, _> = config::parse_toml(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_parse_toml_str() {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct SimpleConfig {
            key: String,
        }

        let content = r#"key = "value""#;
        let config: SimpleConfig = config::parse_toml_str(content, Path::new("test.toml")).unwrap();
        assert_eq!(config.key, "value");
    }

    #[test]
    fn test_config_parse_toml_str_invalid() {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct DummyConfig {
            key: String,
        }

        let content = "invalid = {";
        let result: Result<DummyConfig, _> =
            config::parse_toml_str(content, Path::new("test.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_config_write_toml() {
        use serde::Serialize;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("output.toml");

        #[derive(Serialize)]
        struct WriteConfig {
            name: String,
            version: i32,
        }

        let cfg = WriteConfig {
            name: "test".to_string(),
            version: 1,
        };

        config::write_toml(&config_path, &cfg).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("name = \"test\""));
        assert!(content.contains("version = 1"));
    }

    // ==========================================================================
    // Backup Module Tests
    // ==========================================================================

    #[test]
    fn test_backup_list_empty() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");

        let backups = backup::list_backups(&file_path).unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn test_backup_multiple_backups() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("file.txt");

        fs::write(&original, "v1").unwrap();
        let backup1 = backup::create(&original).unwrap();

        // Wait 1 second to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_secs(1));

        fs::write(&original, "v2").unwrap();
        let backup2 = backup::create(&original).unwrap();

        // Check that two different backups exist
        assert!(backup1.exists());
        assert!(backup2.exists());
        assert_ne!(backup1, backup2);
    }

    #[test]
    fn test_backup_creates_valid_backup() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("file.txt");

        fs::write(&original, "test content").unwrap();
        let backup_path = backup::create(&original).unwrap();

        assert!(backup_path.exists());
        let content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(content, "test content");
    }

    // ==========================================================================
    // Path Module Tests
    // ==========================================================================

    #[test]
    fn test_path_expand_home_no_tilde() {
        let path = "/absolute/path";
        assert_eq!(path::expand_home(path), path);
    }

    #[test]
    fn test_path_expand_env_vars_no_vars() {
        let path = "/plain/path";
        assert_eq!(path::expand_env_vars(path), PathBuf::from("/plain/path"));
    }

    #[test]
    fn test_path_expand_env_vars_missing_var() {
        // SAFETY: Test isolation
        unsafe {
            std::env::remove_var("NONEXISTENT_VAR");
        }
        let result = path::expand_env_vars("${NONEXISTENT_VAR}/path");
        // Should keep the var as-is or return empty
        assert!(!result.to_string_lossy().is_empty());
    }

    #[test]
    fn test_path_normalize_absolute() {
        let path = Path::new("/a/b/c");
        assert_eq!(path::normalize(path), PathBuf::from("/a/b/c"));
    }

    #[test]
    fn test_path_normalize_complex() {
        let path = Path::new("/a/b/./c/../d");
        assert_eq!(path::normalize(path), PathBuf::from("/a/b/d"));
    }

    #[test]
    fn test_path_is_within_same_path() {
        let root = Path::new("/home/user");
        assert!(path::is_within(root, root));
    }

    // ==========================================================================
    // Traverse Module Tests
    // ==========================================================================

    #[test]
    fn test_traverse_options_default() {
        let opts = traverse::TraverseOptions::default();
        assert!(opts.extensions.is_empty());
        assert!(!opts.include_hidden);
        assert!(!opts.follow_symlinks);
    }

    #[test]
    fn test_traverse_find_files_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let files =
            traverse::find_files(temp_dir.path(), &traverse::TraverseOptions::default()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_traverse_find_files_with_depth() {
        let temp_dir = TempDir::new().unwrap();

        // Create structure:
        // root/          (walkdir depth 0)
        //   a.txt        (walkdir depth 1)
        //   level1/      (walkdir depth 1)
        //     b.txt      (walkdir depth 2)
        //     level2/    (walkdir depth 2)
        //       c.txt    (walkdir depth 3)
        fs::write(temp_dir.path().join("a.txt"), "").unwrap();
        fs::create_dir_all(temp_dir.path().join("level1/level2")).unwrap();
        fs::write(temp_dir.path().join("level1/b.txt"), "").unwrap();
        fs::write(temp_dir.path().join("level1/level2/c.txt"), "").unwrap();

        // With max_depth=2, should find a.txt and b.txt (not c.txt at depth 3)
        let opts = traverse::TraverseOptions {
            max_depth: Some(2),
            ..Default::default()
        };

        let files = traverse::find_files(temp_dir.path(), &opts).unwrap();
        assert_eq!(files.len(), 2);

        // With max_depth=1, should only find a.txt
        let opts_shallow = traverse::TraverseOptions {
            max_depth: Some(1),
            ..Default::default()
        };

        let files_shallow = traverse::find_files(temp_dir.path(), &opts_shallow).unwrap();
        assert_eq!(files_shallow.len(), 1);
    }

    #[test]
    fn test_traverse_find_files_multiple_extensions() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("a.txt"), "").unwrap();
        fs::write(temp_dir.path().join("b.toml"), "").unwrap();
        fs::write(temp_dir.path().join("c.rs"), "").unwrap();

        let opts = traverse::TraverseOptions {
            extensions: vec!["txt".to_string(), "toml".to_string()],
            ..Default::default()
        };

        let files = traverse::find_files(temp_dir.path(), &opts).unwrap();
        assert_eq!(files.len(), 2);
    }

    // ==========================================================================
    // Atomic Write Tests
    // ==========================================================================

    #[test]
    fn test_atomic_write_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        atomic_write(&file_path, b"original").unwrap();
        atomic_write(&file_path, b"updated").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "updated");
    }

    #[test]
    fn test_atomic_write_empty_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");

        atomic_write(&file_path, b"").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_atomic_write_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.dat");

        let binary_data: Vec<u8> = (0..255).collect();
        atomic_write(&file_path, &binary_data).unwrap();

        let content = fs::read(&file_path).unwrap();
        assert_eq!(content, binary_data);
    }
}
