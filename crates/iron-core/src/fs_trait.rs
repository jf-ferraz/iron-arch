//! Filesystem Abstraction Layer
//!
//! Provides a trait-based abstraction over filesystem operations to enable
//! isolated testing of services without actual filesystem I/O.
//!
//! # Usage
//!
//! Services can use `FileSystem` trait objects for dependency injection:
//!
//! ```ignore
//! use iron_core::fs_trait::{FileSystem, RealFileSystem};
//!
//! struct MyService<F: FileSystem> {
//!     fs: F,
//! }
//!
//! impl<F: FileSystem> MyService<F> {
//!     fn read_config(&self, path: &Path) -> Result<String, FsError> {
//!         self.fs.read_to_string(path)
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::FsError;

/// Result type for filesystem operations
pub type FsResult<T> = Result<T, FsError>;

/// Abstract filesystem operations for testability
pub trait FileSystem: Send + Sync {
    /// Read file contents as a string
    fn read_to_string(&self, path: &Path) -> FsResult<String>;

    /// Write string contents to a file
    fn write(&self, path: &Path, contents: &str) -> FsResult<()>;

    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if path is a file
    fn is_file(&self, path: &Path) -> bool;

    /// Check if path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Check if path is a symbolic link
    fn is_symlink(&self, path: &Path) -> bool;

    /// Create directory and all parent directories
    fn create_dir_all(&self, path: &Path) -> FsResult<()>;

    /// Remove a file
    fn remove_file(&self, path: &Path) -> FsResult<()>;

    /// Remove a directory (must be empty)
    fn remove_dir(&self, path: &Path) -> FsResult<()>;

    /// Rename/move a file
    fn rename(&self, from: &Path, to: &Path) -> FsResult<()>;

    /// List directory contents (returns names only)
    fn read_dir(&self, path: &Path) -> FsResult<Vec<String>>;

    /// Create a symbolic link
    fn symlink(&self, src: &Path, dst: &Path) -> FsResult<()>;

    /// Read symlink target
    fn read_link(&self, path: &Path) -> FsResult<PathBuf>;

    /// Copy a file
    fn copy(&self, from: &Path, to: &Path) -> FsResult<()>;
}

// =============================================================================
// Real Filesystem Implementation
// =============================================================================

/// Real filesystem implementation using std::fs
#[derive(Debug, Clone, Default)]
pub struct RealFileSystem;

impl RealFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> FsResult<String> {
        std::fs::read_to_string(path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => FsError::NotFound {
                path: path.to_path_buf(),
            },
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })
    }

    fn write(&self, path: &Path, contents: &str) -> FsResult<()> {
        std::fs::write(path, contents).map_err(|e| match e.kind() {
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_symlink(&self, path: &Path) -> bool {
        path.is_symlink()
    }

    fn create_dir_all(&self, path: &Path) -> FsResult<()> {
        std::fs::create_dir_all(path).map_err(|e| match e.kind() {
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })
    }

    fn remove_file(&self, path: &Path) -> FsResult<()> {
        std::fs::remove_file(path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => FsError::NotFound {
                path: path.to_path_buf(),
            },
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })
    }

    fn remove_dir(&self, path: &Path) -> FsResult<()> {
        std::fs::remove_dir(path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => FsError::NotFound {
                path: path.to_path_buf(),
            },
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })
    }

    fn rename(&self, from: &Path, to: &Path) -> FsResult<()> {
        std::fs::rename(from, to).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => FsError::NotFound {
                path: from.to_path_buf(),
            },
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: from.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })
    }

    fn read_dir(&self, path: &Path) -> FsResult<Vec<String>> {
        let entries = std::fs::read_dir(path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => FsError::NotFound {
                path: path.to_path_buf(),
            },
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })?;

        let mut names = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }

    fn symlink(&self, src: &Path, dst: &Path) -> FsResult<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(src, dst).map_err(|e| match e.kind() {
                io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                    path: dst.to_path_buf(),
                },
                io::ErrorKind::AlreadyExists => FsError::AlreadyExists {
                    path: dst.to_path_buf(),
                },
                _ => FsError::IoError {
                    message: e.to_string(),
                },
            })
        }

        #[cfg(not(unix))]
        {
            Err(FsError::IoError {
                message: "Symlinks not supported on this platform".to_string(),
            })
        }
    }

    fn read_link(&self, path: &Path) -> FsResult<PathBuf> {
        std::fs::read_link(path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => FsError::NotFound {
                path: path.to_path_buf(),
            },
            _ => FsError::IoError {
                message: e.to_string(),
            },
        })
    }

    fn copy(&self, from: &Path, to: &Path) -> FsResult<()> {
        std::fs::copy(from, to)
            .map(|_| ())
            .map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => FsError::NotFound {
                    path: from.to_path_buf(),
                },
                io::ErrorKind::PermissionDenied => FsError::PermissionDenied {
                    path: to.to_path_buf(),
                },
                _ => FsError::IoError {
                    message: e.to_string(),
                },
            })
    }
}

// =============================================================================
// Mock Filesystem Implementation
// =============================================================================

/// Entry type in mock filesystem
#[derive(Debug, Clone)]
pub enum MockEntry {
    /// A file with content
    File(String),
    /// A directory
    Dir,
    /// A symbolic link to another path
    Symlink(PathBuf),
}

/// Mock filesystem for isolated testing
///
/// Thread-safe implementation using RwLock for interior mutability.
#[derive(Debug, Clone)]
pub struct MockFileSystem {
    entries: Arc<RwLock<HashMap<PathBuf, MockEntry>>>,
    /// Simulated errors for specific paths
    errors: Arc<RwLock<HashMap<PathBuf, FsError>>>,
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl MockFileSystem {
    /// Create a new empty mock filesystem
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            errors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a file with content
    pub fn add_file(&self, path: impl AsRef<Path>, content: impl Into<String>) -> &Self {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            self.ensure_dirs(parent);
        }

        let mut entries = self.entries.write().unwrap();
        entries.insert(path, MockEntry::File(content.into()));
        self
    }

    /// Add an empty directory
    pub fn add_dir(&self, path: impl AsRef<Path>) -> &Self {
        self.ensure_dirs(path.as_ref());
        self
    }

    /// Add a symbolic link
    pub fn add_symlink(&self, path: impl AsRef<Path>, target: impl AsRef<Path>) -> &Self {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            self.ensure_dirs(parent);
        }

        let mut entries = self.entries.write().unwrap();
        entries.insert(path, MockEntry::Symlink(target.as_ref().to_path_buf()));
        self
    }

    /// Set an error to be returned for operations on a path
    pub fn set_error(&self, path: impl AsRef<Path>, error: FsError) -> &Self {
        let mut errors = self.errors.write().unwrap();
        errors.insert(path.as_ref().to_path_buf(), error);
        self
    }

    /// Clear a previously set error
    pub fn clear_error(&self, path: impl AsRef<Path>) -> &Self {
        let mut errors = self.errors.write().unwrap();
        errors.remove(path.as_ref());
        self
    }

    /// Get file content (for test assertions)
    pub fn get_content(&self, path: impl AsRef<Path>) -> Option<String> {
        let entries = self.entries.read().unwrap();
        match entries.get(path.as_ref()) {
            Some(MockEntry::File(content)) => Some(content.clone()),
            _ => None,
        }
    }

    /// Check if a path was created (for test assertions)
    pub fn has_path(&self, path: impl AsRef<Path>) -> bool {
        let entries = self.entries.read().unwrap();
        entries.contains_key(path.as_ref())
    }

    /// Get all paths in the mock filesystem (for debugging)
    pub fn all_paths(&self) -> Vec<PathBuf> {
        let entries = self.entries.read().unwrap();
        entries.keys().cloned().collect()
    }

    /// Ensure all directories in a path exist
    fn ensure_dirs(&self, path: &Path) {
        let mut entries = self.entries.write().unwrap();
        let mut current = PathBuf::new();

        for component in path.components() {
            current.push(component);
            entries
                .entry(current.clone())
                .or_insert(MockEntry::Dir);
        }
    }

    /// Check for simulated errors
    fn check_error(&self, path: &Path) -> FsResult<()> {
        let errors = self.errors.read().unwrap();
        if let Some(error) = errors.get(path) {
            return Err(error.clone());
        }
        Ok(())
    }
}

impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> FsResult<String> {
        self.check_error(path)?;

        let entries = self.entries.read().unwrap();
        match entries.get(path) {
            Some(MockEntry::File(content)) => Ok(content.clone()),
            Some(MockEntry::Symlink(target)) => {
                // Clone target before dropping to avoid borrow issues
                let target = target.clone();
                drop(entries);
                self.read_to_string(&target)
            }
            Some(MockEntry::Dir) => Err(FsError::IoError {
                message: "Is a directory".to_string(),
            }),
            None => Err(FsError::NotFound {
                path: path.to_path_buf(),
            }),
        }
    }

    fn write(&self, path: &Path, contents: &str) -> FsResult<()> {
        self.check_error(path)?;

        // Ensure parent directories exist
        if let Some(parent) = path.parent() {
            self.ensure_dirs(parent);
        }

        let mut entries = self.entries.write().unwrap();
        entries.insert(path.to_path_buf(), MockEntry::File(contents.to_string()));
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let entries = self.entries.read().unwrap();
        entries.contains_key(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        let entries = self.entries.read().unwrap();
        matches!(entries.get(path), Some(MockEntry::File(_)))
    }

    fn is_dir(&self, path: &Path) -> bool {
        let entries = self.entries.read().unwrap();
        matches!(entries.get(path), Some(MockEntry::Dir))
    }

    fn is_symlink(&self, path: &Path) -> bool {
        let entries = self.entries.read().unwrap();
        matches!(entries.get(path), Some(MockEntry::Symlink(_)))
    }

    fn create_dir_all(&self, path: &Path) -> FsResult<()> {
        self.check_error(path)?;
        self.ensure_dirs(path);
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> FsResult<()> {
        self.check_error(path)?;

        let mut entries = self.entries.write().unwrap();
        match entries.get(path) {
            Some(MockEntry::File(_)) | Some(MockEntry::Symlink(_)) => {
                entries.remove(path);
                Ok(())
            }
            Some(MockEntry::Dir) => Err(FsError::IoError {
                message: "Is a directory".to_string(),
            }),
            None => Err(FsError::NotFound {
                path: path.to_path_buf(),
            }),
        }
    }

    fn remove_dir(&self, path: &Path) -> FsResult<()> {
        self.check_error(path)?;

        let entries_read = self.entries.read().unwrap();

        // Check if directory exists
        if !matches!(entries_read.get(path), Some(MockEntry::Dir)) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        // Check if directory is empty
        let has_children = entries_read.keys().any(|p| {
            p != path && p.starts_with(path)
        });

        if has_children {
            return Err(FsError::IoError {
                message: "Directory not empty".to_string(),
            });
        }

        drop(entries_read);

        let mut entries = self.entries.write().unwrap();
        entries.remove(path);
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> FsResult<()> {
        self.check_error(from)?;
        self.check_error(to)?;

        let mut entries = self.entries.write().unwrap();

        if let Some(entry) = entries.remove(from) {
            // Ensure parent directories exist
            if let Some(parent) = to.parent() {
                drop(entries);
                self.ensure_dirs(parent);
                entries = self.entries.write().unwrap();
            }
            entries.insert(to.to_path_buf(), entry);
            Ok(())
        } else {
            Err(FsError::NotFound {
                path: from.to_path_buf(),
            })
        }
    }

    fn read_dir(&self, path: &Path) -> FsResult<Vec<String>> {
        self.check_error(path)?;

        let entries = self.entries.read().unwrap();

        if !matches!(entries.get(path), Some(MockEntry::Dir)) {
            return Err(FsError::NotFound {
                path: path.to_path_buf(),
            });
        }

        let mut children = Vec::new();
        for entry_path in entries.keys() {
            if let Some(parent) = entry_path.parent() {
                if parent == path {
                    if let Some(name) = entry_path.file_name() {
                        if let Some(name_str) = name.to_str() {
                            children.push(name_str.to_string());
                        }
                    }
                }
            }
        }

        Ok(children)
    }

    fn symlink(&self, src: &Path, dst: &Path) -> FsResult<()> {
        self.check_error(dst)?;

        // Ensure parent directories exist
        if let Some(parent) = dst.parent() {
            self.ensure_dirs(parent);
        }

        let mut entries = self.entries.write().unwrap();

        if entries.contains_key(dst) {
            return Err(FsError::AlreadyExists {
                path: dst.to_path_buf(),
            });
        }

        entries.insert(dst.to_path_buf(), MockEntry::Symlink(src.to_path_buf()));
        Ok(())
    }

    fn read_link(&self, path: &Path) -> FsResult<PathBuf> {
        self.check_error(path)?;

        let entries = self.entries.read().unwrap();
        match entries.get(path) {
            Some(MockEntry::Symlink(target)) => Ok(target.clone()),
            Some(_) => Err(FsError::IoError {
                message: "Not a symbolic link".to_string(),
            }),
            None => Err(FsError::NotFound {
                path: path.to_path_buf(),
            }),
        }
    }

    fn copy(&self, from: &Path, to: &Path) -> FsResult<()> {
        self.check_error(from)?;
        self.check_error(to)?;

        let content = self.read_to_string(from)?;
        self.write(to, &content)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // MockFileSystem Basic Tests
    // =========================================================================

    #[test]
    fn test_mock_add_and_read_file() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "hello world");

        let content = fs.read_to_string(Path::new("/test/file.txt")).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_mock_file_not_found() {
        let fs = MockFileSystem::new();
        let result = fs.read_to_string(Path::new("/nonexistent"));
        assert!(matches!(result, Err(FsError::NotFound { .. })));
    }

    #[test]
    fn test_mock_write_creates_file() {
        let fs = MockFileSystem::new();
        fs.write(Path::new("/new/file.txt"), "content").unwrap();

        assert!(fs.is_file(Path::new("/new/file.txt")));
        assert_eq!(fs.get_content("/new/file.txt"), Some("content".to_string()));
    }

    #[test]
    fn test_mock_exists() {
        let fs = MockFileSystem::new();
        fs.add_file("/exists.txt", "");

        assert!(fs.exists(Path::new("/exists.txt")));
        assert!(!fs.exists(Path::new("/not-exists.txt")));
    }

    #[test]
    fn test_mock_is_file_and_dir() {
        let fs = MockFileSystem::new();
        fs.add_file("/file.txt", "content");
        fs.add_dir("/dir");

        assert!(fs.is_file(Path::new("/file.txt")));
        assert!(!fs.is_file(Path::new("/dir")));
        assert!(fs.is_dir(Path::new("/dir")));
        assert!(!fs.is_dir(Path::new("/file.txt")));
    }

    #[test]
    fn test_mock_create_dir_all() {
        let fs = MockFileSystem::new();
        fs.create_dir_all(Path::new("/a/b/c/d")).unwrap();

        assert!(fs.is_dir(Path::new("/a")));
        assert!(fs.is_dir(Path::new("/a/b")));
        assert!(fs.is_dir(Path::new("/a/b/c")));
        assert!(fs.is_dir(Path::new("/a/b/c/d")));
    }

    #[test]
    fn test_mock_remove_file() {
        let fs = MockFileSystem::new();
        fs.add_file("/file.txt", "content");

        assert!(fs.exists(Path::new("/file.txt")));
        fs.remove_file(Path::new("/file.txt")).unwrap();
        assert!(!fs.exists(Path::new("/file.txt")));
    }

    #[test]
    fn test_mock_rename() {
        let fs = MockFileSystem::new();
        fs.add_file("/old.txt", "content");

        fs.rename(Path::new("/old.txt"), Path::new("/new.txt")).unwrap();

        assert!(!fs.exists(Path::new("/old.txt")));
        assert!(fs.exists(Path::new("/new.txt")));
        assert_eq!(fs.get_content("/new.txt"), Some("content".to_string()));
    }

    #[test]
    fn test_mock_read_dir() {
        let fs = MockFileSystem::new();
        fs.add_dir("/parent");
        fs.add_file("/parent/a.txt", "");
        fs.add_file("/parent/b.txt", "");
        fs.add_dir("/parent/subdir");

        let mut children = fs.read_dir(Path::new("/parent")).unwrap();
        children.sort();

        assert_eq!(children, vec!["a.txt", "b.txt", "subdir"]);
    }

    // =========================================================================
    // Symlink Tests
    // =========================================================================

    #[test]
    fn test_mock_symlink() {
        let fs = MockFileSystem::new();
        fs.add_file("/target.txt", "target content");
        fs.symlink(Path::new("/target.txt"), Path::new("/link.txt")).unwrap();

        assert!(fs.is_symlink(Path::new("/link.txt")));

        let target = fs.read_link(Path::new("/link.txt")).unwrap();
        assert_eq!(target, PathBuf::from("/target.txt"));
    }

    #[test]
    fn test_mock_symlink_already_exists() {
        let fs = MockFileSystem::new();
        fs.add_file("/existing.txt", "");

        let result = fs.symlink(Path::new("/target"), Path::new("/existing.txt"));
        assert!(matches!(result, Err(FsError::AlreadyExists { .. })));
    }

    #[test]
    fn test_mock_read_through_symlink() {
        let fs = MockFileSystem::new();
        fs.add_file("/target.txt", "symlinked content");
        fs.add_symlink("/link.txt", "/target.txt");

        let content = fs.read_to_string(Path::new("/link.txt")).unwrap();
        assert_eq!(content, "symlinked content");
    }

    // =========================================================================
    // Error Simulation Tests
    // =========================================================================

    #[test]
    fn test_mock_simulated_permission_error() {
        let fs = MockFileSystem::new();
        fs.add_file("/protected.txt", "content");
        fs.set_error("/protected.txt", FsError::PermissionDenied {
            path: PathBuf::from("/protected.txt"),
        });

        let result = fs.read_to_string(Path::new("/protected.txt"));
        assert!(matches!(result, Err(FsError::PermissionDenied { .. })));
    }

    #[test]
    fn test_mock_clear_error() {
        let fs = MockFileSystem::new();
        fs.add_file("/file.txt", "content");
        fs.set_error("/file.txt", FsError::PermissionDenied {
            path: PathBuf::from("/file.txt"),
        });

        // Error active
        assert!(fs.read_to_string(Path::new("/file.txt")).is_err());

        // Clear error
        fs.clear_error("/file.txt");
        let content = fs.read_to_string(Path::new("/file.txt")).unwrap();
        assert_eq!(content, "content");
    }

    // =========================================================================
    // Copy Tests
    // =========================================================================

    #[test]
    fn test_mock_copy() {
        let fs = MockFileSystem::new();
        fs.add_file("/source.txt", "source content");

        fs.copy(Path::new("/source.txt"), Path::new("/dest.txt")).unwrap();

        assert!(fs.is_file(Path::new("/dest.txt")));
        assert_eq!(fs.get_content("/dest.txt"), Some("source content".to_string()));
    }

    // =========================================================================
    // RealFileSystem Tests (with tempdir)
    // =========================================================================

    #[test]
    fn test_real_fs_read_write() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let fs = RealFileSystem::new();
        fs.write(&file_path, "test content").unwrap();

        let content = fs.read_to_string(&file_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_real_fs_create_dir_all() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("a/b/c/d");

        let fs = RealFileSystem::new();
        fs.create_dir_all(&nested_dir).unwrap();

        assert!(fs.is_dir(&nested_dir));
    }

    #[test]
    fn test_real_fs_symlink() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let target = temp_dir.path().join("target.txt");
        let link = temp_dir.path().join("link.txt");

        let fs = RealFileSystem::new();
        fs.write(&target, "target content").unwrap();
        fs.symlink(&target, &link).unwrap();

        assert!(fs.is_symlink(&link));
        let read_target = fs.read_link(&link).unwrap();
        assert_eq!(read_target, target);
    }

    // =========================================================================
    // Thread Safety Tests
    // =========================================================================

    #[test]
    fn test_mock_thread_safe() {
        use std::thread;

        let fs = Arc::new(MockFileSystem::new());
        let mut handles = Vec::new();

        for i in 0..10 {
            let fs_clone = Arc::clone(&fs);
            handles.push(thread::spawn(move || {
                fs_clone.add_file(format!("/file_{}.txt", i), format!("content {}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all files were created
        for i in 0..10 {
            assert!(fs.exists(Path::new(&format!("/file_{}.txt", i))));
        }
    }
}
