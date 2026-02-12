//! Iron Pacman - Package manager integration
//!
//! Handles:
//! - Package installation/removal
//! - AUR helper integration (paru/yay)
//! - Update risk assessment
//! - Breaking change detection

use anyhow::Result;
use std::process::Command;

/// Risk level for updates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Update preview information
#[derive(Debug)]
pub struct UpdatePreview {
    pub packages: Vec<PackageUpdate>,
    pub arch_news: Vec<ArchNewsItem>,
    pub risk_level: RiskLevel,
    pub risk_reasons: Vec<String>,
}

/// Package update details
#[derive(Debug)]
pub struct PackageUpdate {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub is_aur: bool,
    pub is_flagged: bool,
}

/// Arch News item
#[derive(Debug)]
pub struct ArchNewsItem {
    pub title: String,
    pub date: String,
    pub url: String,
    pub requires_manual: bool,
}

/// Check for available updates
pub fn check_updates() -> Result<Vec<PackageUpdate>> {
    // Run checkupdates
    let output = Command::new("checkupdates")
        .output()?;

    let mut updates = Vec::new();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                updates.push(PackageUpdate {
                    name: parts[0].to_string(),
                    current_version: parts[1].to_string(),
                    new_version: parts[3].to_string(),
                    is_aur: false,
                    is_flagged: false,
                });
            }
        }
    }

    Ok(updates)
}

/// Calculate risk level for updates
pub fn assess_risk(updates: &[PackageUpdate], news: &[ArchNewsItem]) -> (RiskLevel, Vec<String>) {
    let mut reasons = Vec::new();
    let mut risk = RiskLevel::Low;

    // Check for kernel updates
    if updates.iter().any(|u| u.name.starts_with("linux")) {
        reasons.push("Kernel update detected".to_string());
        risk = RiskLevel::Medium;
    }

    // Check for nvidia driver updates
    if updates.iter().any(|u| u.name.starts_with("nvidia")) {
        reasons.push("NVIDIA driver update".to_string());
        risk = RiskLevel::Medium;
    }

    // Check for manual intervention news
    if news.iter().any(|n| n.requires_manual) {
        reasons.push("Manual intervention may be required".to_string());
        risk = RiskLevel::High;
    }

    // Check for flagged AUR packages
    if updates.iter().any(|u| u.is_flagged) {
        reasons.push("Flagged AUR package".to_string());
        risk = RiskLevel::High;
    }

    (risk, reasons)
}

/// Fetch Arch Linux news
pub fn fetch_arch_news() -> Result<Vec<ArchNewsItem>> {
    // TODO: Implement RSS feed parsing
    Ok(Vec::new())
}

/// Install packages
pub fn install_packages(packages: &[String], aur_helper: &str) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let mut cmd = Command::new(aur_helper);
    cmd.arg("-S").arg("--needed");
    cmd.args(packages);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("Package installation failed");
    }

    Ok(())
}

/// Remove packages
pub fn remove_packages(packages: &[String]) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let mut cmd = Command::new("pacman");
    cmd.arg("-Rns");
    cmd.args(packages);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("Package removal failed");
    }

    Ok(())
}

/// Detect AUR helper
pub fn detect_aur_helper() -> Option<String> {
    for helper in ["paru", "yay", "pikaur", "trizen"] {
        if Command::new("which").arg(helper).output().is_ok() {
            return Some(helper.to_string());
        }
    }
    None
}
