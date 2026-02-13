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
    fn test_list_keys_empty() {
        let (service, _temp) = create_test_service();

        let keys = service.list_keys().unwrap();
        assert!(keys.is_empty());
    }
}
