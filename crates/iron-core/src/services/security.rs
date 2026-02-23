//! Security Service — Security level calculator
//!
//! F2-016: SecurityLevel enum + SecurityService trait
//! F2-019: Uses Module::security_points for scoring

use crate::IronResult;
use crate::module::Module;
use crate::services::state::StateManager;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Security hardening level based on enabled modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SecurityLevel {
    /// 0-20 points
    Basic,
    /// 21-50 points
    Standard,
    /// 51-80 points
    Advanced,
    /// 81+ points
    Paranoid,
}

impl SecurityLevel {
    /// Determine level from a point score.
    pub fn from_score(score: u32) -> Self {
        match score {
            0..=20 => SecurityLevel::Basic,
            21..=50 => SecurityLevel::Standard,
            51..=80 => SecurityLevel::Advanced,
            _ => SecurityLevel::Paranoid,
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            SecurityLevel::Basic => "Basic",
            SecurityLevel::Standard => "Standard",
            SecurityLevel::Advanced => "Advanced",
            SecurityLevel::Paranoid => "Paranoid",
        }
    }
}

/// Information about a security-contributing module.
#[derive(Debug, Clone, Serialize)]
pub struct SecurityModuleInfo {
    pub id: String,
    pub name: String,
    pub points: u32,
    pub enabled: bool,
}

/// Full security report.
#[derive(Debug, Clone, Serialize)]
pub struct SecurityReport {
    pub level: SecurityLevel,
    pub score: u32,
    pub max_score: u32,
    pub enabled_modules: Vec<SecurityModuleInfo>,
    pub available_modules: Vec<SecurityModuleInfo>,
    pub recommendations: Vec<String>,
}

/// Service for calculating security posture.
pub trait SecurityService: Send + Sync {
    fn calculate(&self) -> IronResult<SecurityReport>;
}

/// Default implementation scanning modules directory.
pub struct DefaultSecurityService {
    iron_root: PathBuf,
    state_manager: StateManager,
}

impl DefaultSecurityService {
    pub fn new(iron_root: &Path, state_manager: StateManager) -> Self {
        Self {
            iron_root: iron_root.to_path_buf(),
            state_manager,
        }
    }
}

impl SecurityService for DefaultSecurityService {
    fn calculate(&self) -> IronResult<SecurityReport> {
        let modules_dir = self.iron_root.join("modules");
        let active_modules = self.state_manager.active_modules();

        let mut enabled_modules = Vec::new();
        let mut available_modules = Vec::new();
        let mut score: u32 = 0;
        let mut max_score: u32 = 0;

        // Scan all modules for security_points > 0
        if let Ok(entries) = std::fs::read_dir(&modules_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Ok(module) = Module::load(&path)
                    && module.security_points > 0
                {
                    let is_enabled = active_modules.contains(&module.id);
                    let info = SecurityModuleInfo {
                        id: module.id.clone(),
                        name: module.name.clone(),
                        points: module.security_points,
                        enabled: is_enabled,
                    };

                    max_score += module.security_points;

                    if is_enabled {
                        score += module.security_points;
                        enabled_modules.push(info);
                    } else {
                        available_modules.push(info);
                    }
                }
            }
        }

        // Also check well-known security module IDs even if they don't exist yet
        let well_known = well_known_security_modules();
        for (id, name, points) in &well_known {
            let already_counted = enabled_modules.iter().any(|m| m.id == *id)
                || available_modules.iter().any(|m| m.id == *id);
            if !already_counted {
                max_score += points;
                available_modules.push(SecurityModuleInfo {
                    id: id.to_string(),
                    name: name.to_string(),
                    points: *points,
                    enabled: false,
                });
            }
        }

        // Sort available modules by points descending before generating recommendations
        available_modules.sort_by(|a, b| b.points.cmp(&a.points));

        // Generate recommendations (already in highest-points-first order)
        let mut recommendations = Vec::new();
        let level = SecurityLevel::from_score(score);

        for module in &available_modules {
            let new_level = SecurityLevel::from_score(score + module.points);
            if new_level != level {
                recommendations.push(format!(
                    "Enable '{}' for +{} points → {} level",
                    module.id,
                    module.points,
                    new_level.label()
                ));
            } else {
                recommendations.push(format!(
                    "Enable '{}' for +{} points",
                    module.id, module.points
                ));
            }
        }

        Ok(SecurityReport {
            level,
            score,
            max_score,
            enabled_modules,
            available_modules,
            recommendations,
        })
    }
}

/// Well-known security module IDs with default point values.
fn well_known_security_modules() -> Vec<(&'static str, &'static str, u32)> {
    vec![
        ("ufw", "UFW Firewall", 10),
        ("firewalld", "Firewalld", 10),
        ("fail2ban", "Fail2ban", 10),
        ("ssh-hardening", "SSH Hardening", 10),
        ("apparmor", "AppArmor", 15),
        ("sandboxing", "Sandboxing", 15),
        ("auditd", "Audit Daemon", 10),
        ("intrusion-detection", "Intrusion Detection", 15),
        ("kernel-hardening", "Kernel Hardening", 15),
        ("password-policy", "Password Policy", 5),
        ("dns-security", "DNS Security", 10),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_level_basic() {
        assert_eq!(SecurityLevel::from_score(0), SecurityLevel::Basic);
        assert_eq!(SecurityLevel::from_score(10), SecurityLevel::Basic);
        assert_eq!(SecurityLevel::from_score(20), SecurityLevel::Basic);
    }

    #[test]
    fn test_security_level_standard() {
        assert_eq!(SecurityLevel::from_score(21), SecurityLevel::Standard);
        assert_eq!(SecurityLevel::from_score(35), SecurityLevel::Standard);
        assert_eq!(SecurityLevel::from_score(50), SecurityLevel::Standard);
    }

    #[test]
    fn test_security_level_advanced() {
        assert_eq!(SecurityLevel::from_score(51), SecurityLevel::Advanced);
        assert_eq!(SecurityLevel::from_score(65), SecurityLevel::Advanced);
        assert_eq!(SecurityLevel::from_score(80), SecurityLevel::Advanced);
    }

    #[test]
    fn test_security_level_paranoid() {
        assert_eq!(SecurityLevel::from_score(81), SecurityLevel::Paranoid);
        assert_eq!(SecurityLevel::from_score(100), SecurityLevel::Paranoid);
        assert_eq!(SecurityLevel::from_score(200), SecurityLevel::Paranoid);
    }

    #[test]
    fn test_security_level_labels() {
        assert_eq!(SecurityLevel::Basic.label(), "Basic");
        assert_eq!(SecurityLevel::Standard.label(), "Standard");
        assert_eq!(SecurityLevel::Advanced.label(), "Advanced");
        assert_eq!(SecurityLevel::Paranoid.label(), "Paranoid");
    }

    #[test]
    fn test_well_known_modules_nonempty() {
        let modules = well_known_security_modules();
        assert!(!modules.is_empty());
        // Each module has non-zero points
        for (_, _, points) in &modules {
            assert!(*points > 0);
        }
    }

    #[test]
    fn test_calculate_empty_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = StateManager::new(tmp.path().to_path_buf()).unwrap();
        let svc = DefaultSecurityService::new(tmp.path(), state);

        let report = svc.calculate().unwrap();
        assert_eq!(report.level, SecurityLevel::Basic);
        assert_eq!(report.score, 0);
        assert!(report.max_score > 0); // well-known modules contribute
        assert!(report.enabled_modules.is_empty());
        assert!(!report.available_modules.is_empty());
    }

    #[test]
    fn test_security_report_serializable() {
        let report = SecurityReport {
            level: SecurityLevel::Standard,
            score: 30,
            max_score: 100,
            enabled_modules: vec![SecurityModuleInfo {
                id: "ufw".into(),
                name: "UFW Firewall".into(),
                points: 10,
                enabled: true,
            }],
            available_modules: vec![],
            recommendations: vec!["Enable fail2ban for +10 points".into()],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("Standard"));
        assert!(json.contains("ufw"));
    }
}
