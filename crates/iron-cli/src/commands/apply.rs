//! Iron Apply Command
//!
//! Converge system to declared state from host.toml.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::apply::{ApplyPlan, ApplyService, PrunePolicy, RiskLevel};

/// Execute apply command
#[allow(clippy::too_many_arguments)]
pub fn execute(
    ctx: &AppContext,
    dry_run: bool,
    module: Option<&str>,
    yes: bool,
    prune: bool,
    prune_packages: bool,
    prune_services: bool,
    prune_dotfiles: bool,
    force_hooks: bool,
) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;

    // F3-014: Build prune policy from CLI flags
    let prune_policy = if prune {
        PrunePolicy::all()
    } else {
        PrunePolicy {
            packages: prune_packages,
            services: prune_services,
            dotfiles: prune_dotfiles,
        }
    };

    let service = ctx
        .apply_service()
        .with_prune_policy(prune_policy)
        .with_force_hooks(force_hooks)
        .with_interactive(!yes);

    output.header("Iron Apply");

    // Compute plan
    let plan = if let Some(mod_id) = module {
        output.info(&format!("Computing plan for module '{}'...", mod_id));
        service.plan_module(mod_id)?
    } else {
        let host_id = ctx.current_host().unwrap_or_else(|| "default".to_string());
        if !dry_run && !output.is_json() {
            let spinner =
                crate::progress::ProgressReporter::spinner("Computing system apply plan...");
            let result = service.plan(&host_id);
            spinner.finish("Plan computed");
            result?
        } else {
            output.info("Computing system apply plan...");
            service.plan(&host_id)?
        }
    };

    if plan.is_empty() {
        output.success("System is already in desired state — nothing to do ✓");
        return Ok(());
    }

    // Display plan with risk summary
    display_plan_with_risk(output, &plan);

    // F3-014: Hint about prunable actions when pruning is off
    let prune_count = plan.prune_count();
    let has_prune = prune || prune_packages || prune_services || prune_dotfiles;
    if prune_count > 0 && !has_prune {
        output.info(&format!(
            "  {} removal action(s) shown but will not execute. \
             Use --prune to include.",
            prune_count
        ));
    }

    if dry_run {
        // Show what confirmation WOULD look like
        display_dry_run_confirmation(output, &plan, yes);
        output.success("[DRY RUN] No changes made.");
        return Ok(());
    }

    // F3-016: Risk-scaled confirmation
    if !confirm_apply(output, &plan, yes)? {
        output.info("Cancelled.");
        return Ok(());
    }

    // Execute
    let spinner = if !output.is_json() {
        Some(crate::progress::ProgressReporter::spinner(
            "Applying changes...",
        ))
    } else {
        None
    };
    let result = service.execute(&plan)?;
    if let Some(s) = spinner {
        s.finish("Apply complete");
    }

    output.separator();
    for (i, action) in plan.actions.iter().enumerate() {
        if i < result.succeeded {
            output.list_item_status(&action.display(), StatusBadge::Ok);
        } else {
            output.list_item_status(&action.display(), StatusBadge::Error);
        }
    }

    for error in &result.errors {
        output.error(error);
    }

    output.separator();
    output.success(&format!(
        "Apply complete: {} succeeded, {} failed ({:.1}s)",
        result.succeeded, result.failed, result.duration_secs
    ));

    output.summary(&[("succeeded", result.succeeded), ("failed", result.failed)]);

    Ok(())
}

/// Display the plan grouped by actions with a risk badge header.
fn display_plan_with_risk(output: &crate::output::Output, plan: &ApplyPlan) {
    let max_risk = plan.max_risk();
    let risk_badge = match max_risk {
        RiskLevel::ReadOnly => "[SAFE]",
        RiskLevel::Additive => "[SAFE]",
        RiskLevel::Destructive => "[CAUTION]",
        RiskLevel::Critical => "[DANGER]",
    };

    output.subheader(&format!("Plan {}", risk_badge));
    for action in &plan.actions {
        output.info(&format!("  {}", action.display()));
    }
    output.separator();

    // Risk summary line
    let risk_counts = plan.risk_summary();
    let mut risk_parts = Vec::new();
    for level in &[
        RiskLevel::Additive,
        RiskLevel::Destructive,
        RiskLevel::Critical,
    ] {
        if let Some(&count) = risk_counts.get(level) {
            risk_parts.push(format!("{} {}", count, level));
        }
    }
    if !risk_parts.is_empty() {
        output.info(&format!(
            "Summary: {} ({})",
            plan.summary(),
            risk_parts.join(", ")
        ));
    } else {
        output.info(&format!("Summary: {}", plan.summary()));
    }
}

/// Show what the confirmation prompt WOULD be (for --dry-run).
fn display_dry_run_confirmation(output: &crate::output::Output, plan: &ApplyPlan, yes: bool) {
    let max_risk = plan.max_risk();
    match max_risk {
        RiskLevel::ReadOnly => {
            output.info("[dry-run] No confirmation needed (read-only).");
        }
        RiskLevel::Additive => {
            if yes {
                output.info("[dry-run] Would auto-confirm (--yes).");
            } else {
                output.info("[dry-run] Would prompt: Proceed? [y/N]");
            }
        }
        RiskLevel::Destructive => {
            if yes {
                output.info("[dry-run] Would auto-confirm (--yes).");
            } else {
                output.info(
                    "[dry-run] Would prompt: \
                     This will modify/remove files. \
                     Proceed? [y/N]",
                );
            }
        }
        RiskLevel::Critical => {
            output.info(
                "[dry-run] Would prompt: \
                 Type 'yes' to confirm critical changes: \
                 (--yes does NOT bypass this)",
            );
        }
    }
}

/// Risk-scaled confirmation. Returns true if the user confirms.
fn confirm_apply(output: &crate::output::Output, plan: &ApplyPlan, yes: bool) -> Result<bool> {
    let max_risk = plan.max_risk();
    match max_risk {
        RiskLevel::ReadOnly => Ok(true),
        RiskLevel::Additive => {
            if yes {
                return Ok(true);
            }
            output.info("Proceed? [y/N]");
            read_yes_no()
        }
        RiskLevel::Destructive => {
            if yes {
                return Ok(true);
            }
            output.info("This will modify/remove files. Proceed? [y/N]");
            read_yes_no()
        }
        RiskLevel::Critical => {
            if yes {
                output.warning(
                    "Critical changes detected. \
                     --yes does NOT bypass this.",
                );
            }
            output.warning("Type 'yes' to confirm critical changes:");
            read_typed_yes()
        }
    }
}

/// Read a y/N confirmation from stdin.
fn read_yes_no() -> Result<bool> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Read a typed "yes" confirmation from stdin.
fn read_typed_yes() -> Result<bool> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim() == "yes")
}

#[cfg(test)]
mod tests {
    use iron_core::services::apply::{ApplyAction, ApplyPlan, RiskLevel};
    use std::collections::HashMap;

    #[test]
    fn test_plan_summary_format() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["neovim".into()],
                },
                ApplyAction::ActivateModule { id: "nvim".into() },
            ],
        };
        assert!(!plan.is_empty());
        assert!(plan.summary().contains("+1 pkg"));
    }

    #[test]
    fn test_empty_plan() {
        let plan = ApplyPlan::default();
        assert!(plan.is_empty());
    }

    #[test]
    fn test_max_risk_additive_plan() {
        let plan = ApplyPlan {
            actions: vec![ApplyAction::InstallPackages {
                packages: vec!["a".into()],
            }],
        };
        assert_eq!(plan.max_risk(), RiskLevel::Additive);
    }

    #[test]
    fn test_max_risk_destructive_plan() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::RenderAndCopy {
                    source: "s".into(),
                    target: "t".into(),
                    variables: HashMap::new(),
                    module_id: "m".into(),
                },
            ],
        };
        assert_eq!(plan.max_risk(), RiskLevel::Destructive);
    }

    #[test]
    fn test_max_risk_critical_plan() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::RemovePackages {
                    packages: vec!["b".into()],
                },
            ],
        };
        assert_eq!(plan.max_risk(), RiskLevel::Critical);
    }

    #[test]
    fn test_risk_summary_in_plan() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["a".into()],
                },
                ApplyAction::EnableService { name: "svc".into() },
                ApplyAction::RemovePackages {
                    packages: vec!["b".into()],
                },
            ],
        };
        let summary = plan.risk_summary();
        assert_eq!(summary.get(&RiskLevel::Additive), Some(&2));
        assert_eq!(summary.get(&RiskLevel::Critical), Some(&1));
    }

    #[test]
    fn test_empty_plan_is_readonly() {
        let plan = ApplyPlan::default();
        assert_eq!(plan.max_risk(), RiskLevel::ReadOnly);
    }
}
