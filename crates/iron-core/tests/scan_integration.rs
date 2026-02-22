//! Integration tests for the ScanService
//!
//! These tests verify that the system scanner correctly discovers
//! configs, detects conflicts, and generates recommendations when
//! run against real filesystem structures.

use iron_core::packages::NoopPackageManager;
use iron_core::services::scan::{DefaultScanService, ScanService};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper: create a scan service pointing at a temp home dir.
fn scan_service(home: &Path) -> DefaultScanService {
    DefaultScanService::new(home, Arc::new(NoopPackageManager))
}

// ============================================================================
// Empty System Tests
// ============================================================================

#[test]
fn test_scan_empty_home_no_bundles() {
    let home = TempDir::new().unwrap();
    let svc = scan_service(home.path());

    let report = svc.scan(&[], &[]).unwrap();

    assert!(report.existing_configs.is_empty());
    assert!(report.installed_packages.is_empty());
    assert!(report.potential_conflicts.is_empty());
    assert_eq!(report.summary.configs_scanned, 0);
    assert_eq!(report.summary.conflicts_found, 0);
}

// ============================================================================
// Config Discovery Tests
// ============================================================================

#[test]
fn test_scan_discovers_xdg_configs() {
    let home = TempDir::new().unwrap();
    let xdg_config = home.path().join(".config");
    std::fs::create_dir_all(xdg_config.join("nvim")).unwrap();
    std::fs::create_dir_all(xdg_config.join("kitty")).unwrap();

    let svc = scan_service(home.path());
    let report = svc.scan(&[], &[]).unwrap();

    let app_names: Vec<&str> = report
        .existing_configs
        .iter()
        .map(|c| c.app_name.as_str())
        .collect();
    assert!(app_names.contains(&"Neovim"));
    assert!(app_names.contains(&"Kitty terminal"));
    assert!(report.summary.configs_scanned >= 2);
}

#[test]
fn test_scan_discovers_home_dotfiles() {
    let home = TempDir::new().unwrap();
    std::fs::write(home.path().join(".bashrc"), "# shell config").unwrap();
    std::fs::write(home.path().join(".gitconfig"), "[user]\n").unwrap();

    let svc = scan_service(home.path());
    let report = svc.scan(&[], &[]).unwrap();

    let app_names: Vec<&str> = report
        .existing_configs
        .iter()
        .map(|c| c.app_name.as_str())
        .collect();
    assert!(app_names.contains(&"Bash shell"));
    assert!(app_names.contains(&"Git"));
}

#[test]
fn test_scan_detects_symlinks() {
    let home = TempDir::new().unwrap();
    let xdg_config = home.path().join(".config");
    let target_dir = home.path().join("real-nvim-config");

    std::fs::create_dir_all(&xdg_config).unwrap();
    std::fs::create_dir_all(&target_dir).unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_dir, xdg_config.join("nvim")).unwrap();

    let svc = scan_service(home.path());
    let report = svc.scan(&[], &[]).unwrap();

    let nvim = report
        .existing_configs
        .iter()
        .find(|c| c.app_name == "Neovim");
    assert!(nvim.is_some(), "Should find nvim config");
    let nvim = nvim.unwrap();
    assert!(nvim.is_symlink, "Should detect symlink");
    assert!(nvim.symlink_target.is_some());
}

// ============================================================================
// Recommendation Tests
// ============================================================================

#[test]
fn test_scan_generates_backup_recommendation_for_existing_configs() {
    let home = TempDir::new().unwrap();
    let xdg_config = home.path().join(".config");
    std::fs::create_dir_all(xdg_config.join("nvim")).unwrap();
    std::fs::create_dir_all(xdg_config.join("kitty")).unwrap();
    std::fs::create_dir_all(xdg_config.join("fish")).unwrap();

    let svc = scan_service(home.path());
    let report = svc.scan(&[], &[]).unwrap();

    // With multiple configs, should recommend backing up
    assert!(
        !report.recommendations.is_empty(),
        "Should generate recommendations for existing configs"
    );
}

// ============================================================================
// Report Consistency Tests
// ============================================================================

#[test]
fn test_scan_summary_matches_details() {
    let home = TempDir::new().unwrap();
    let xdg_config = home.path().join(".config");
    std::fs::create_dir_all(xdg_config.join("nvim")).unwrap();
    std::fs::write(home.path().join(".bashrc"), "").unwrap();

    let svc = scan_service(home.path());
    let report = svc.scan(&[], &[]).unwrap();

    assert_eq!(
        report.summary.configs_scanned,
        report.existing_configs.len()
    );
    assert_eq!(
        report.summary.conflicts_found,
        report.potential_conflicts.len()
    );
    assert_eq!(
        report.summary.recommendations_count,
        report.recommendations.len()
    );
    assert_eq!(
        report.summary.packages_already_installed,
        report.installed_packages.len()
    );
}

// ============================================================================
// JSON Serialization Tests
// ============================================================================

#[test]
fn test_scan_report_serializes_to_json() {
    let home = TempDir::new().unwrap();
    let xdg_config = home.path().join(".config");
    std::fs::create_dir_all(xdg_config.join("nvim")).unwrap();

    let svc = scan_service(home.path());
    let report = svc.scan(&[], &[]).unwrap();

    let json = serde_json::to_string_pretty(&report).unwrap();
    assert!(json.contains("existing_configs"));
    assert!(json.contains("summary"));

    // Round-trip
    let deserialized: iron_core::services::scan::ScanReport =
        serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.existing_configs.len(),
        report.existing_configs.len()
    );
}
