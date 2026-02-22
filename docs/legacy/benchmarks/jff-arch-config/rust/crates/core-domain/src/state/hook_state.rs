use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Hook execution behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookBehavior {
    /// Always ask before execution
    Ask,
    /// Always execute without asking
    Always,
    /// Execute once, then skip
    Once,
    /// Never execute
    Skip,
}

impl Default for HookBehavior {
    fn default() -> Self {
        HookBehavior::Ask
    }
}

impl HookBehavior {
    pub fn from_str(s: &str) -> Option<HookBehavior> {
        match s.to_lowercase().as_str() {
            "ask" => Some(HookBehavior::Ask),
            "always" => Some(HookBehavior::Always),
            "once" => Some(HookBehavior::Once),
            "skip" => Some(HookBehavior::Skip),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HookBehavior::Ask => "ask",
            HookBehavior::Always => "always",
            HookBehavior::Once => "once",
            HookBehavior::Skip => "skip",
        }
    }
}

/// State for a hook script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookState {
    pub hook_id: String,
    pub script_path: PathBuf,
    pub script_hash: String,
    pub behavior: HookBehavior,
    pub last_executed: Option<DateTime<Utc>>,
    pub execution_count: usize,
}

impl HookState {
    pub fn new(hook_id: String, script_path: PathBuf, behavior: HookBehavior) -> io::Result<Self> {
        let hash = compute_file_hash(&script_path)?;
        Ok(HookState {
            hook_id,
            script_path,
            script_hash: hash,
            behavior,
            last_executed: None,
            execution_count: 0,
        })
    }

    pub fn should_execute(&self) -> bool {
        match self.behavior {
            HookBehavior::Skip => false,
            HookBehavior::Always => true,
            HookBehavior::Ask => true,
            HookBehavior::Once => self.execution_count == 0,
        }
    }

    pub fn has_changed(&self) -> io::Result<bool> {
        let current_hash = compute_file_hash(&self.script_path)?;
        Ok(current_hash != self.script_hash)
    }

    pub fn update_hash(&mut self) -> io::Result<()> {
        self.script_hash = compute_file_hash(&self.script_path)?;
        Ok(())
    }

    pub fn record_execution(&mut self) {
        self.last_executed = Some(Utc::now());
        self.execution_count += 1;
    }
}

/// Compute SHA-256 hash of a file
pub fn compute_file_hash(path: &Path) -> io::Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}
