//! Secrets Service - Secure secret management with git-crypt
//!
//! Provides git-crypt integration for encrypted secrets in repository.
//! Supports an optional `SecretsBackend` trait object so that a resilient
//! backend (e.g., `iron-git::DefaultSecretsManager` with circuit breaker)
//! can be injected without creating a circular dependency.

use crate::{IronResult, ServiceError};
use crate::state::OperationStatus;
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

// =========================================================================
// SecretsBackend — low-level git-crypt operations (defined in core,
// implemented by callers such as iron-git::DefaultSecretsManager)
// =========================================================================

/// Low-level secrets backend for git-crypt operations.
///
/// This trait allows `DefaultSecretsService` to delegate the overlapping
/// operations (unlock, lock, is_unlocked, list_encrypted) to a dedicated
/// backend that may add circuit-breaker / timeout / retry logic.
///
/// When no backend is injected the service falls back to direct `Command`
/// execution (the original behaviour).
pub trait SecretsBackend: Send + Sync {
    /// Check whether secrets are currently unlocked (decrypted).
    fn is_unlocked(&self) -> bool;

    /// Unlock (decrypt) secrets, optionally using a symmetric key file.
    fn unlock(&self, key_path: Option<&Path>) -> IronResult<()>;

    /// Lock (re-encrypt) secrets.
    fn lock(&self) -> IronResult<()>;

    /// List encrypted file paths.
    fn list_encrypted(&self) -> IronResult<Vec<PathBuf>>;
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
    /// Optional state manager for audit logging
    state_manager: Option<crate::services::state::StateManager>,
    /// Optional resilient backend for git-crypt operations
    backend: Option<Box<dyn SecretsBackend>>,
}

impl DefaultSecretsService {
    /// Create a new secrets service
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            state_manager: None,
            backend: None,
        }
    }

    /// Add state manager for audit logging (builder pattern).
    pub fn with_state_manager(mut self, sm: crate::services::state::StateManager) -> Self {
        self.state_manager = Some(sm);
        self
    }

    /// Inject a resilient secrets backend (builder pattern).
    ///
    /// When set, `unlock`, `lock`, status-detection (is_unlocked) and
    /// `list_encrypted` delegate to the backend instead of spawning
    /// `Command` directly.
    pub fn with_backend(mut self, backend: Box<dyn SecretsBackend>) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Record an operation if a state manager is available.
    fn audit(&self, operation: &str, status: OperationStatus, details: Option<String>) {
        if let Some(ref sm) = self.state_manager {
            let _ = sm.record_operation(operation, status, details);
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
        // Delegate to backend if available
        if let Some(ref backend) = self.backend {
            return backend.is_unlocked();
        }
        // Fallback: Check by looking for the .git/git-crypt/keys directory
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
        self.audit("secrets_init", OperationStatus::Success, None);
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

        // Delegate to backend if available
        if let Some(ref backend) = self.backend {
            backend.unlock(key_path)?;
        } else {
            let result = if let Some(key) = key_path {
                self.git_crypt(&["unlock", key.to_str().unwrap_or("")])
            } else {
                // Use GPG key
                self.git_crypt(&["unlock"])
            };
            result.map(|_| ())?;
        }

        self.audit("secrets_unlock", OperationStatus::Success, None);
        Ok(())
    }

    fn lock(&self) -> IronResult<()> {
        if !self.is_initialized() {
            return Err(ServiceError::OperationFailed {
                service: "git-crypt".to_string(),
                message: "Repository not initialized with git-crypt".to_string(),
            }
            .into());
        }

        // Delegate to backend if available
        if let Some(ref backend) = self.backend {
            backend.lock()?;
        } else {
            self.git_crypt(&["lock"])?;
        }

        self.audit("secrets_lock", OperationStatus::Success, None);
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
        self.audit(
            "secrets_add_gpg_user",
            OperationStatus::Success,
            Some(format!("key_id={}", key_id)),
        );
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
        // Delegate to backend if available
        if let Some(ref backend) = self.backend {
            return backend.list_encrypted();
        }

        // Fallback: parse .gitattributes for git-crypt patterns
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

        // Collect all files in secrets/ directory
        let mut encrypted_files = Vec::new();
        let secrets_dir = self.repo_root.join("secrets");

        if secrets_dir.exists() {
            for entry in walkdir::WalkDir::new(&secrets_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path().to_path_buf();

                // If we have patterns from .gitattributes, filter by them
                if !encrypted_patterns.is_empty() {
                    let relative = path
                        .strip_prefix(&self.repo_root)
                        .unwrap_or(&path)
                        .to_string_lossy();
                    if encrypted_patterns
                        .iter()
                        .any(|pat| glob_match(pat, &relative))
                    {
                        encrypted_files.push(path);
                    }
                } else {
                    // No patterns found — assume all secrets/ files are encrypted
                    encrypted_files.push(path);
                }
            }
        }

        Ok(encrypted_files)
    }
}

/// Simple glob pattern matching for gitattributes patterns.
///
/// Supports `*` (matches any sequence except `/`) and `**` (matches any sequence including `/`).
/// Patterns without `/` are matched against the basename only (gitattributes convention).
fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");

    // gitattributes: if pattern has no '/', match against the basename only
    if !pattern.contains('/') {
        let basename = path.rsplit('/').next().unwrap_or(&path);
        return glob_match_inner(pattern.as_bytes(), basename.as_bytes());
    }

    glob_match_inner(pattern.as_bytes(), path.as_bytes())
}

/// Core byte-level glob matching engine (`*` stops at `/`, `**` crosses `/`, `?` matches one byte).
fn glob_match_inner(pat: &[u8], txt: &[u8]) -> bool {
    let (mut pi, mut ti) = (0, 0);
    let (mut star_pi, mut star_ti) = (usize::MAX, 0);

    while ti < txt.len() {
        if pi < pat.len() && pat[pi] == b'*' {
            // Check for **
            if pi + 1 < pat.len() && pat[pi + 1] == b'*' {
                // ** matches everything including /
                star_pi = pi;
                star_ti = ti;
                pi += 2;
                // Skip optional trailing /
                if pi < pat.len() && pat[pi] == b'/' {
                    pi += 1;
                }
                continue;
            }
            // Single * matches everything except /
            star_pi = pi;
            star_ti = ti;
            pi += 1;
            continue;
        }

        if pi < pat.len() && (pat[pi] == b'?' || pat[pi] == txt[ti]) {
            pi += 1;
            ti += 1;
            continue;
        }

        // Single * cannot match /
        if star_pi != usize::MAX
            && star_pi < pat.len()
            && pat[star_pi] == b'*'
            && !(star_pi + 1 < pat.len() && pat[star_pi + 1] == b'*')
        {
            // Single * — skip non-/ characters
            if txt[star_ti] == b'/' {
                // Cannot match / with single *, backtrack fails
                return false;
            }
            star_ti += 1;
            ti = star_ti;
            pi = star_pi + 1;
            continue;
        }

        if star_pi != usize::MAX {
            star_ti += 1;
            ti = star_ti;
            pi = if pat[star_pi] == b'*' && star_pi + 1 < pat.len() && pat[star_pi + 1] == b'*' {
                star_pi
                    + 2
                    + if star_pi + 2 < pat.len() && pat[star_pi + 2] == b'/' {
                        1
                    } else {
                        0
                    }
            } else {
                star_pi + 1
            };
            continue;
        }

        return false;
    }

    // Consume remaining pattern characters (must all be *)
    while pi < pat.len() && pat[pi] == b'*' {
        pi += 1;
    }

    pi == pat.len()
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

    // ==========================================================================
    // glob_match Tests
    // ==========================================================================

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("secrets/keys.gpg", "secrets/keys.gpg"));
        assert!(!glob_match("secrets/keys.gpg", "secrets/other.gpg"));
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("secrets/*.gpg", "secrets/keys.gpg"));
        assert!(glob_match("secrets/*.gpg", "secrets/other.gpg"));
        assert!(!glob_match("secrets/*.gpg", "secrets/sub/keys.gpg"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("secrets/**", "secrets/keys.gpg"));
        assert!(glob_match("secrets/**", "secrets/sub/keys.gpg"));
        assert!(!glob_match("secrets/**", "other/keys.gpg"));
    }

    // ==========================================================================
    // list_encrypted filtering Tests
    // ==========================================================================

    #[test]
    fn test_list_encrypted_filters_by_pattern() {
        let (service, temp_dir) = create_test_service();

        // Create .gitattributes with a pattern
        std::fs::write(
            temp_dir.path().join(".gitattributes"),
            "secrets/*.key filter=git-crypt diff=git-crypt\n",
        )
        .unwrap();

        // Create files in secrets/
        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();
        std::fs::write(secrets_dir.join("prod.key"), "secret").unwrap();
        std::fs::write(secrets_dir.join("readme.md"), "not secret").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("prod.key"));
    }

    #[test]
    fn test_list_encrypted_no_patterns_returns_all() {
        let (service, temp_dir) = create_test_service();

        // No .gitattributes file
        let secrets_dir = temp_dir.path().join("secrets");
        std::fs::create_dir_all(&secrets_dir).unwrap();
        std::fs::write(secrets_dir.join("prod.key"), "secret").unwrap();
        std::fs::write(secrets_dir.join("readme.md"), "also here").unwrap();

        let files = service.list_encrypted().unwrap();
        assert_eq!(files.len(), 2);
    }

    // ==========================================================================
    // Audit Logging Tests (S1-P9-005)
    // ==========================================================================

    #[test]
    fn test_with_state_manager_builder() {
        let temp_dir = TempDir::new().unwrap();
        let sm = crate::services::state::StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let service = DefaultSecretsService::new(temp_dir.path()).with_state_manager(sm);
        assert!(service.state_manager.is_some());
    }

    #[test]
    fn test_audit_records_when_state_manager_present() {
        let temp_dir = TempDir::new().unwrap();
        let sm = crate::services::state::StateManager::new(temp_dir.path().to_path_buf()).unwrap();
        let service = DefaultSecretsService::new(temp_dir.path()).with_state_manager(sm.clone());

        // Directly call audit and verify via state manager
        service.audit("test_op", OperationStatus::Success, Some("detail".to_string()));

        let recent = sm.recent_audit(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].operation, "test_op");
    }

    #[test]
    fn test_audit_noop_without_state_manager() {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultSecretsService::new(temp_dir.path());
        // Should not panic
        service.audit("test_op", OperationStatus::Success, None);
    }

    #[test]
    fn test_default_service_has_no_state_manager() {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultSecretsService::new(temp_dir.path());
        assert!(service.state_manager.is_none());
    }

    // ==========================================================================
    // SecretsBackend delegation tests
    // ==========================================================================

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Mock backend that records which operations were called.
    struct MockBackend {
        unlock_called: Arc<AtomicBool>,
        lock_called: Arc<AtomicBool>,
        is_unlocked_value: bool,
    }

    impl MockBackend {
        fn new(is_unlocked: bool) -> Self {
            Self {
                unlock_called: Arc::new(AtomicBool::new(false)),
                lock_called: Arc::new(AtomicBool::new(false)),
                is_unlocked_value: is_unlocked,
            }
        }
    }

    impl SecretsBackend for MockBackend {
        fn is_unlocked(&self) -> bool {
            self.is_unlocked_value
        }

        fn unlock(&self, _key_path: Option<&Path>) -> IronResult<()> {
            self.unlock_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn lock(&self) -> IronResult<()> {
            self.lock_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn list_encrypted(&self) -> IronResult<Vec<PathBuf>> {
            Ok(vec![PathBuf::from("secrets/mock.enc")])
        }
    }

    #[test]
    fn test_with_backend_builder() {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultSecretsService::new(temp_dir.path())
            .with_backend(Box::new(MockBackend::new(true)));
        assert!(service.backend.is_some());
    }

    #[test]
    fn test_default_service_has_no_backend() {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultSecretsService::new(temp_dir.path());
        assert!(service.backend.is_none());
    }

    #[test]
    fn test_status_delegates_is_unlocked_to_backend() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();
        let service = DefaultSecretsService::new(temp_dir.path())
            .with_backend(Box::new(MockBackend::new(true)));
        assert!(service.is_unlocked());
    }

    #[test]
    fn test_status_delegates_locked_to_backend() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();
        let service = DefaultSecretsService::new(temp_dir.path())
            .with_backend(Box::new(MockBackend::new(false)));
        assert!(!service.is_unlocked());
    }

    #[test]
    fn test_list_encrypted_delegates_to_backend() {
        let temp_dir = TempDir::new().unwrap();
        let service = DefaultSecretsService::new(temp_dir.path())
            .with_backend(Box::new(MockBackend::new(true)));
        let files = service.list_encrypted().unwrap();
        assert_eq!(files, vec![PathBuf::from("secrets/mock.enc")]);
    }

    #[test]
    fn test_unlock_delegates_to_backend() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();
        let unlock_flag = Arc::new(AtomicBool::new(false));
        let backend = MockBackend {
            unlock_called: Arc::clone(&unlock_flag),
            lock_called: Arc::new(AtomicBool::new(false)),
            is_unlocked_value: false,
        };
        let service = DefaultSecretsService::new(temp_dir.path())
            .with_backend(Box::new(backend));
        service.unlock(None).unwrap();
        assert!(unlock_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_lock_delegates_to_backend() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(temp_dir.path().join(".git-crypt")).unwrap();
        let lock_flag = Arc::new(AtomicBool::new(false));
        let backend = MockBackend {
            unlock_called: Arc::new(AtomicBool::new(false)),
            lock_called: Arc::clone(&lock_flag),
            is_unlocked_value: true,
        };
        let service = DefaultSecretsService::new(temp_dir.path())
            .with_backend(Box::new(backend));
        service.lock().unwrap();
        assert!(lock_flag.load(Ordering::SeqCst));
    }
}
