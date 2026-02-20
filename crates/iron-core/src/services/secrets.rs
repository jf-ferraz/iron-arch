//! Secrets Service - Secure secret management with git-crypt
//!
//! Provides git-crypt integration for encrypted secrets in repository.

use crate::{IronResult, ServiceError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Secrets encryption status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretsStatus {
    /// Repository not initialized for git-crypt
    NotInitialized,
    /// Secrets are encrypted (locked)
    Locked,
    /// Secrets are decrypted (unlocked)
    Unlocked,
    /// git-crypt not available
    NotAvailable,
}

/// GPG key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpgKey {
    /// Key ID
    pub id: String,
    /// User ID (email)
    pub user_id: String,
    /// Trust level
    pub trust: String,
}

/// Secrets service trait
pub trait SecretsService {
    /// Get secrets encryption status
    fn status(&self) -> IronResult<SecretsStatus>;

    /// Initialize git-crypt in repository
    fn init(&self) -> IronResult<()>;

    /// Unlock secrets (decrypt)
    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()>;

    /// Lock secrets (re-encrypt)
    fn lock(&self) -> IronResult<()>;

    /// Add GPG user to repository
    fn add_gpg_user(&self, key_id: &str) -> IronResult<()>;

    /// List authorized GPG keys
    fn list_keys(&self) -> IronResult<Vec<GpgKey>>;

    /// Export symmetric key
    fn export_key(&self, output_path: &Path) -> IronResult<()>;

    /// Check if file is encrypted
    fn is_encrypted(&self, file: &Path) -> bool;

    /// List encrypted files
    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>>;
}

/// Default secrets service implementation
pub struct DefaultSecretsService {
    /// Repository root
    repo_root: PathBuf,
}

impl DefaultSecretsService {
    /// Create a new secrets service
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
        }
    }

    /// Check if git-crypt is available
    fn git_crypt_available(&self) -> bool {
        Command::new("git-crypt")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run git-crypt command
    fn git_crypt(&self, args: &[&str]) -> IronResult<String> {
        if !self.git_crypt_available() {
            return Err(ServiceError::NotAvailable {
                service: "git-crypt".to_string(),
            }
            .into());
        }

        let output = Command::new("git-crypt")
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|_| ServiceError::NotAvailable {
                service: "git-crypt".to_string(),
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(ServiceError::OperationFailed {
                service: "git-crypt".to_string(),
                message: stderr,
            }
            .into())
        }
    }

    /// Check if repository is initialized with git-crypt
    fn is_initialized(&self) -> bool {
        self.repo_root.join(".git-crypt").exists()
    }

    /// Check if currently unlocked
    fn is_unlocked(&self) -> bool {
        // Check by looking for the .git/git-crypt/keys directory
        // When locked, encrypted files contain the git-crypt header
        let keys_dir = self.repo_root.join(".git").join("git-crypt").join("keys");
        keys_dir.exists()
    }
}

impl SecretsService for DefaultSecretsService {
    fn status(&self) -> IronResult<SecretsStatus> {
        if !self.git_crypt_available() {
            return Ok(SecretsStatus::NotAvailable);
        }

        if !self.is_initialized() {
            return Ok(SecretsStatus::NotInitialized);
        }

        if self.is_unlocked() {
            Ok(SecretsStatus::Unlocked)
        } else {
            Ok(SecretsStatus::Locked)
        }
    }

    fn init(&self) -> IronResult<()> {
        if self.is_initialized() {
            return Err(ServiceError::OperationFailed {
                service: "git-crypt".to_string(),
                message: "Repository already initialized".to_string(),
            }
            .into());
        }

        self.git_crypt(&["init"])?;
        Ok(())
    }

    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()> {
        if !self.is_initialized() {
            return Err(ServiceError::OperationFailed {
                service: "git-crypt".to_string(),
                message: "Repository not initialized with git-crypt".to_string(),
            }
            .into());
        }

        let result = if let Some(key) = key_path {
            self.git_crypt(&["unlock", key.to_str().unwrap_or("")])
        } else {
            // Use GPG key
            self.git_crypt(&["unlock"])
        };

        result.map(|_| ())
    }

    fn lock(&self) -> IronResult<()> {
        if !self.is_initialized() {
            return Err(ServiceError::OperationFailed {
                service: "git-crypt".to_string(),
                message: "Repository not initialized with git-crypt".to_string(),
            }
            .into());
        }

        self.git_crypt(&["lock"])?;
        Ok(())
    }

    fn add_gpg_user(&self, key_id: &str) -> IronResult<()> {
        if !self.is_initialized() {
            return Err(ServiceError::OperationFailed {
                service: "git-crypt".to_string(),
                message: "Repository not initialized with git-crypt".to_string(),
            }
            .into());
        }

        self.git_crypt(&["add-gpg-user", key_id])?;
        Ok(())
    }

    fn list_keys(&self) -> IronResult<Vec<GpgKey>> {
        // List GPG keys that have access
        let gpg_dir = self
            .repo_root
            .join(".git-crypt")
            .join("keys")
            .join("default")
            .join("0");

        if !gpg_dir.exists() {
            return Ok(vec![]);
        }

        let mut keys = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&gpg_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str()
                    && filename.ends_with(".gpg")
                {
                    let key_id = filename.trim_end_matches(".gpg").to_string();
                    // Try to get user info from gpg
                    let user_id = Command::new("gpg")
                        .args(["--list-keys", "--with-colons", &key_id])
                        .output()
                        .ok()
                        .and_then(|o| {
                            String::from_utf8_lossy(&o.stdout)
                                .lines()
                                .find(|l| l.starts_with("uid"))
                                .and_then(|l| l.split(':').nth(9))
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_else(|| "Unknown".to_string());

                    keys.push(GpgKey {
                        id: key_id,
                        user_id,
                        trust: "unknown".to_string(),
                    });
                }
            }
        }

        Ok(keys)
    }

    fn export_key(&self, output_path: &Path) -> IronResult<()> {
        if !self.is_initialized() {
            return Err(ServiceError::OperationFailed {
                service: "git-crypt".to_string(),
                message: "Repository not initialized with git-crypt".to_string(),
            }
            .into());
        }

        self.git_crypt(&["export-key", output_path.to_str().unwrap_or("")])?;
        Ok(())
    }

    fn is_encrypted(&self, file: &Path) -> bool {
        // Check if file starts with git-crypt header
        if let Ok(content) = std::fs::read(file) {
            // git-crypt encrypted files start with "\x00GITCRYPT" (9 bytes)
            content.len() >= 9 && &content[0..9] == b"\x00GITCRYPT"
        } else {
            false
        }
    }

    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>> {
        // Parse .gitattributes for git-crypt patterns
        let gitattributes = self.repo_root.join(".gitattributes");
        let mut encrypted_patterns = Vec::new();

        if let Ok(content) = std::fs::read_to_string(&gitattributes) {
            for line in content.lines() {
                if (line.contains("filter=git-crypt") || line.contains("diff=git-crypt"))
                    && let Some(pattern) = line.split_whitespace().next()
                {
                    encrypted_patterns.push(pattern.to_string());
                }
            }
        }

        // Find files matching patterns (simplified - just check secrets/ directory)
        let mut encrypted_files = Vec::new();
        let secrets_dir = self.repo_root.join("secrets");

        if secrets_dir.exists() {
            for entry in walkdir::WalkDir::new(&secrets_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                encrypted_files.push(entry.path().to_path_buf());
            }
        }

        Ok(encrypted_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_service() -> (DefaultSecretsService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultSecretsService::new(temp_dir.path());
        (service, temp_dir)
    }

    // ==========================================================================
    // SecretsStatus Tests
    // ==========================================================================

    #[test]
    fn test_secrets_status_equality() {
        assert_eq!(SecretsStatus::NotInitialized, SecretsStatus::NotInitialized);
        assert_eq!(SecretsStatus::Locked, SecretsStatus::Locked);
        assert_eq!(SecretsStatus::Unlocked, SecretsStatus::Unlocked);
        assert_eq!(SecretsStatus::NotAvailable, SecretsStatus::NotAvailable);
    }

    #[test]
    fn test_secrets_status_inequality() {
        assert_ne!(SecretsStatus::Locked, SecretsStatus::Unlocked);
        assert_ne!(SecretsStatus::NotInitialized, SecretsStatus::Locked);
        assert_ne!(SecretsStatus::NotAvailable, SecretsStatus::Unlocked);
    }

    #[test]
    fn test_secrets_status_clone() {
        let status = SecretsStatus::Locked;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_secrets_status_copy() {
        let status = SecretsStatus::Unlocked;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn test_secrets_status_debug() {
        let statuses = vec![
            SecretsStatus::NotInitialized,
            SecretsStatus::Locked,
            SecretsStatus::Unlocked,
            SecretsStatus::NotAvailable,
        ];

        for status in statuses {
            let debug_str = format!("{:?}", status);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_secrets_status_serialization() {
        let status = SecretsStatus::Locked;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: SecretsStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_secrets_status_all_variants_serialization() {
        let statuses = vec![
            SecretsStatus::NotInitialized,
            SecretsStatus::Locked,
            SecretsStatus::Unlocked,
            SecretsStatus::NotAvailable,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: SecretsStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    // ==========================================================================
    // GpgKey Tests
    // ==========================================================================

    #[test]
    fn test_gpg_key_creation() {
        let key = GpgKey {
            id: "ABCD1234".to_string(),
            user_id: "user@example.com".to_string(),
            trust: "ultimate".to_string(),
        };

        assert_eq!(key.id, "ABCD1234");
        assert_eq!(key.user_id, "user@example.com");
        assert_eq!(key.trust, "ultimate");
    }

    #[test]
    fn test_gpg_key_clone() {
        let key = GpgKey {
            id: "KEY123".to_string(),
            user_id: "test@test.com".to_string(),
            trust: "full".to_string(),
        };

        let cloned = key.clone();
        assert_eq!(cloned.id, "KEY123");
        assert_eq!(cloned.user_id, "test@test.com");
    }

    #[test]
    fn test_gpg_key_debug() {
        let key = GpgKey {
            id: "TESTKEY".to_string(),
            user_id: "debug@test.com".to_string(),
            trust: "unknown".to_string(),
        };

        let debug_str = format!("{:?}", key);
        assert!(debug_str.contains("TESTKEY"));
        assert!(debug_str.contains("debug@test.com"));
    }

    #[test]
    fn test_gpg_key_serialization() {
        let key = GpgKey {
            id: "SERIAL123".to_string(),
            user_id: "serial@test.com".to_string(),
            trust: "marginal".to_string(),
        };

        let json = serde_json::to_string(&key).unwrap();
        let deserialized: GpgKey = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "SERIAL123");
        assert_eq!(deserialized.user_id, "serial@test.com");
        assert_eq!(deserialized.trust, "marginal");
    }

    #[test]
    fn test_gpg_key_empty_fields() {
        let key = GpgKey {
            id: "".to_string(),
            user_id: "".to_string(),
            trust: "".to_string(),
        };

        assert!(key.id.is_empty());
        assert!(key.user_id.is_empty());
        assert!(key.trust.is_empty());
    }

    // ==========================================================================
    // DefaultSecretsService Tests
    // ==========================================================================

    #[test]
    fn test_status_not_initialized() {
        let (service, _temp) = create_test_service();

        let status = service.status().unwrap();
        // Will be NotAvailable if git-crypt not installed, otherwise NotInitialized
        assert!(status == SecretsStatus::NotAvailable || status == SecretsStatus::NotInitialized);
    }

    #[test]
    fn test_is_initialized() {
        let (service, temp_dir) = create_test_service();

        assert!(!service.is_initialized());

        // Create fake .git-crypt dir
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();

        assert!(service.is_initialized());
    }

    #[test]
    fn test_is_encrypted() {
        let (service, temp_dir) = create_test_service();

        // Create a regular file
        let regular_file = temp_dir.path().join("regular.txt");
        std::fs::write(&regular_file, "regular content").unwrap();
        assert!(!service.is_encrypted(&regular_file));

        // Create a fake encrypted file
        let encrypted_file = temp_dir.path().join("encrypted.txt");
        std::fs::write(&encrypted_file, b"\x00GITCRYPTmore data here").unwrap();
        assert!(service.is_encrypted(&encrypted_file));
    }

    #[test]
    fn test_is_encrypted_nonexistent_file() {
        let (service, temp_dir) = create_test_service();

        let nonexistent = temp_dir.path().join("nonexistent.txt");
        assert!(!service.is_encrypted(&nonexistent));
    }

    #[test]
    fn test_is_encrypted_short_file() {
        let (service, temp_dir) = create_test_service();

        let short_file = temp_dir.path().join("short.txt");
        std::fs::write(&short_file, b"\x00GIT").unwrap(); // Less than 9 bytes
        assert!(!service.is_encrypted(&short_file));
    }

    #[test]
    fn test_is_encrypted_exactly_header() {
        let (service, temp_dir) = create_test_service();

        let exact_file = temp_dir.path().join("exact.txt");
        std::fs::write(&exact_file, b"\x00GITCRYPT").unwrap(); // Exactly 9 bytes
        assert!(service.is_encrypted(&exact_file));
    }

    #[test]
    fn test_list_keys_empty() {
        let (service, _temp) = create_test_service();

        let keys = service.list_keys().unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_list_encrypted_no_secrets_dir() {
        let (service, _temp) = create_test_service();

        let files = service.list_encrypted().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_list_encrypted_empty_secrets_dir() {
        let (service, temp_dir) = create_test_service();

        std::fs::create_dir_all(temp_dir.path().join("secrets")).unwrap();

        let files = service.list_encrypted().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_list_encrypted_with_files() {
        let (service, temp_dir) = create_test_service();

        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();

        std::fs::write(secrets_dir.join("api_key.txt"), "secret").unwrap();
        std::fs::write(secrets_dir.join("database.env"), "password").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_list_encrypted_nested_files() {
        let (service, temp_dir) = create_test_service();

        let secrets_dir = temp_dir.path().join("secrets");
        let nested_dir = secrets_dir.join("production");
        std::fs::create_dir_all(&nested_dir).unwrap();

        std::fs::write(secrets_dir.join("config.txt"), "config").unwrap();
        std::fs::write(nested_dir.join("api_key.txt"), "key").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultSecretsService::new(temp_dir.path());

        // Just verify it creates without panicking
        assert!(!service.is_initialized());
    }

    #[test]
    fn test_service_new_with_existing_gitcrypt() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();

        let service = DefaultSecretsService::new(temp_dir.path());
        assert!(service.is_initialized());
    }

    // ==========================================================================
    // is_unlocked Tests
    // ==========================================================================

    #[test]
    fn test_is_unlocked_no_git_dir() {
        let (service, _temp) = create_test_service();
        assert!(!service.is_unlocked());
    }

    #[test]
    fn test_is_unlocked_with_keys_dir() {
        let (service, temp_dir) = create_test_service();

        // Create the full path that indicates unlocked state
        let keys_dir = temp_dir.path().join(".git").join("git-crypt").join("keys");
        std::fs::create_dir_all(&keys_dir).unwrap();

        assert!(service.is_unlocked());
    }

    #[test]
    fn test_is_unlocked_partial_path() {
        let (service, temp_dir) = create_test_service();

        // Only create .git/git-crypt but not keys dir
        let partial_dir = temp_dir.path().join(".git").join("git-crypt");
        std::fs::create_dir_all(&partial_dir).unwrap();

        assert!(!service.is_unlocked());
    }

    // ==========================================================================
    // status() path tests
    // ==========================================================================

    #[test]
    fn test_status_initialized_but_locked() {
        let (service, temp_dir) = create_test_service();

        // Create .git-crypt (initialized) but not keys dir (locked)
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();

        let status = service.status().unwrap();
        // Will be NotAvailable if git-crypt not installed, otherwise Locked
        assert!(status == SecretsStatus::NotAvailable || status == SecretsStatus::Locked);
    }

    #[test]
    fn test_status_initialized_and_unlocked() {
        let (service, temp_dir) = create_test_service();

        // Create both .git-crypt and keys dir
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join(".git").join("git-crypt").join("keys"))
            .unwrap();

        let status = service.status().unwrap();
        // Will be NotAvailable if git-crypt not installed, otherwise Unlocked
        assert!(status == SecretsStatus::NotAvailable || status == SecretsStatus::Unlocked);
    }

    // ==========================================================================
    // list_encrypted gitattributes parsing tests
    // ==========================================================================

    #[test]
    fn test_list_encrypted_with_gitattributes() {
        let (service, temp_dir) = create_test_service();

        // Create .gitattributes with git-crypt patterns
        let gitattributes = temp_dir.path().join(".gitattributes");
        std::fs::write(
            &gitattributes,
            "secrets/** filter=git-crypt diff=git-crypt\n\
             *.secret filter=git-crypt diff=git-crypt\n",
        )
        .unwrap();

        // Create secrets directory with files
        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();
        std::fs::write(secrets_dir.join("api.key"), "secret").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_list_encrypted_gitattributes_filter_only() {
        let (service, temp_dir) = create_test_service();

        // Create .gitattributes with only filter pattern
        let gitattributes = temp_dir.path().join(".gitattributes");
        std::fs::write(&gitattributes, "secrets/* filter=git-crypt\n").unwrap();

        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();
        std::fs::write(secrets_dir.join("test.txt"), "test").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_list_encrypted_gitattributes_diff_only() {
        let (service, temp_dir) = create_test_service();

        // Create .gitattributes with only diff pattern
        let gitattributes = temp_dir.path().join(".gitattributes");
        std::fs::write(&gitattributes, "*.enc diff=git-crypt\n").unwrap();

        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();
        std::fs::write(secrets_dir.join("config.enc"), "data").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_list_encrypted_no_gitattributes() {
        let (service, temp_dir) = create_test_service();

        // No .gitattributes file, but secrets directory exists
        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();
        std::fs::write(secrets_dir.join("file.txt"), "content").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 1); // Still lists files from secrets dir
    }

    // ==========================================================================
    // Error path tests (without git-crypt installed)
    // ==========================================================================

    #[test]
    fn test_init_already_initialized_error() {
        let (service, temp_dir) = create_test_service();

        // Create .git-crypt to simulate initialized state
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();

        let result = service.init();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already initialized"));
    }

    #[test]
    fn test_unlock_not_initialized_error() {
        let (service, _temp) = create_test_service();

        let result = service.unlock(None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not initialized") || err.contains("not available"),
            "Expected 'not initialized' or 'not available', got: {}",
            err
        );
    }

    #[test]
    fn test_lock_not_initialized_error() {
        let (service, _temp) = create_test_service();

        let result = service.lock();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not initialized") || err.contains("not available"),
            "Expected 'not initialized' or 'not available', got: {}",
            err
        );
    }

    #[test]
    fn test_add_gpg_user_not_initialized_error() {
        let (service, _temp) = create_test_service();

        let result = service.add_gpg_user("ABC123");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not initialized") || err.contains("not available"),
            "Expected 'not initialized' or 'not available', got: {}",
            err
        );
    }

    #[test]
    fn test_export_key_not_initialized_error() {
        let (service, temp_dir) = create_test_service();

        let key_path = temp_dir.path().join("exported.key");
        let result = service.export_key(&key_path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not initialized") || err.contains("not available"),
            "Expected 'not initialized' or 'not available', got: {}",
            err
        );
    }

    // ==========================================================================
    // list_keys with GPG files tests
    // ==========================================================================

    #[test]
    fn test_list_keys_with_gpg_files() {
        let (service, temp_dir) = create_test_service();

        // Create the GPG keys directory structure
        let keys_dir = temp_dir
            .path()
            .join(".git-crypt")
            .join("keys")
            .join("default")
            .join("0");
        std::fs::create_dir_all(&keys_dir).unwrap();

        // Create fake .gpg key files
        std::fs::write(keys_dir.join("ABCD1234.gpg"), "fake key data").unwrap();
        std::fs::write(keys_dir.join("EFGH5678.gpg"), "fake key data 2").unwrap();

        let keys = service.list_keys().unwrap();
        assert_eq!(keys.len(), 2);

        let key_ids: Vec<_> = keys.iter().map(|k| k.id.as_str()).collect();
        assert!(key_ids.contains(&"ABCD1234"));
        assert!(key_ids.contains(&"EFGH5678"));
    }

    #[test]
    fn test_list_keys_ignores_non_gpg_files() {
        let (service, temp_dir) = create_test_service();

        let keys_dir = temp_dir
            .path()
            .join(".git-crypt")
            .join("keys")
            .join("default")
            .join("0");
        std::fs::create_dir_all(&keys_dir).unwrap();

        // Create mixed files
        std::fs::write(keys_dir.join("KEY123.gpg"), "key").unwrap();
        std::fs::write(keys_dir.join("readme.txt"), "not a key").unwrap();
        std::fs::write(keys_dir.join("config"), "config").unwrap();

        let keys = service.list_keys().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].id, "KEY123");
    }

    // ==========================================================================
    // is_encrypted edge cases
    // ==========================================================================

    #[test]
    fn test_is_encrypted_empty_file() {
        let (service, temp_dir) = create_test_service();

        let empty_file = temp_dir.path().join("empty.txt");
        std::fs::write(&empty_file, b"").unwrap();
        assert!(!service.is_encrypted(&empty_file));
    }

    #[test]
    fn test_is_encrypted_binary_but_wrong_header() {
        let (service, temp_dir) = create_test_service();

        let binary_file = temp_dir.path().join("binary.dat");
        std::fs::write(&binary_file, b"\x00NOTCRYPT123456789").unwrap();
        assert!(!service.is_encrypted(&binary_file));
    }

    #[test]
    fn test_is_encrypted_gitcrypt_with_trailing_data() {
        let (service, temp_dir) = create_test_service();

        let enc_file = temp_dir.path().join("encrypted.dat");
        // git-crypt header followed by encrypted binary data
        let mut data = Vec::from(b"\x00GITCRYPT".as_slice());
        data.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05]);
        std::fs::write(&enc_file, data).unwrap();
        assert!(service.is_encrypted(&enc_file));
    }

    #[test]
    fn test_is_encrypted_directory() {
        let (service, temp_dir) = create_test_service();

        let dir = temp_dir.path().join("subdir");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!service.is_encrypted(&dir));
    }
}
