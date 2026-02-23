//! Iron Plan Command
//!
//! F3-005: Preview what `iron apply` would do, without executing.
//! Read-only: no confirmation prompt, no side effects.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::apply::{ApplyAction, ApplyService, PrunePolicy};
use std::time::Instant;

/// Execute plan command
#[allow(clippy::too_many_arguments)]
pub fn execute(
    ctx: &AppContext,
    module: Option<&str>,
    dry_run: bool,
    prune: bool,
    prune_packages: bool,
    prune_services: bool,
    prune_dotfiles: bool,
) -> Result<()> {
    let start = Instant::now();
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

    let service = ctx.apply_service().with_prune_policy(prune_policy);

    // Compute plan
    let plan = if dry_run {
        // In dry-run mode, return empty plan to avoid system queries
        output.info("[DRY RUN] Skipping system scan.");
        iron_core::services::apply::ApplyPlan::default()
    } else if let Some(mod_id) = module {
        output.info(&format!("Computing plan for module '{}'...", mod_id));
        service.plan_module(mod_id)?
    } else {
        let host_id = ctx.current_host().unwrap_or_else(|| "default".to_string());
        if !output.is_json() {
            let spinner = crate::progress::ProgressReporter::spinner("Computing system plan...");
            let result = service.plan(&host_id);
            spinner.finish("Plan computed");
            result?
        } else {
            service.plan(&host_id)?
        }
    };

    // JSON envelope output
    if output.is_json() {
        output.json_envelope("plan", &plan, start);
        return Ok(());
    }

    if plan.is_empty() {
        output.header("Iron Plan");
        output.success("System is already in desired state -- nothing to do.");
        return Ok(());
    }

    output.header("Iron Plan");
    output.info(&format!(
        "{} action(s) to converge system:",
        plan.action_count()
    ));
    output.separator();

    // Group actions by type for clear display
    let pkg_install: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                ApplyAction::InstallPackages { .. } | ApplyAction::InstallAurPackages { .. }
            )
        })
        .collect();
    let pkg_remove: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| matches!(a, ApplyAction::RemovePackages { .. }))
        .collect();
    let dotfile_actions: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                ApplyAction::CreateSymlink { .. }
                    | ApplyAction::RenderAndCopy { .. }
                    | ApplyAction::CopyFile { .. }
            )
        })
        .collect();
    let dotfile_remove: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| matches!(a, ApplyAction::RemoveSymlink { .. }))
        .collect();
    let svc_enable: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| matches!(a, ApplyAction::EnableService { .. }))
        .collect();
    let svc_disable: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| matches!(a, ApplyAction::DisableService { .. }))
        .collect();
    let mod_activate: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| matches!(a, ApplyAction::ActivateModule { .. }))
        .collect();
    let mod_deactivate: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| matches!(a, ApplyAction::DeactivateModule { .. }))
        .collect();

    if !pkg_install.is_empty() || !pkg_remove.is_empty() {
        output.subheader("Packages");
        for action in &pkg_install {
            match action {
                ApplyAction::InstallPackages { packages: pkgs } => {
                    for pkg in pkgs {
                        output.list_item_status(
                            &format!("{} {}", output.colored("+", "\x1b[32m"), pkg),
                            StatusBadge::Ok,
                        );
                    }
                }
                ApplyAction::InstallAurPackages { packages: pkgs } => {
                    for pkg in pkgs {
                        output.list_item_status(
                            &format!("{} {} (AUR)", output.colored("+", "\x1b[32m"), pkg),
                            StatusBadge::Ok,
                        );
                    }
                }
                _ => {}
            }
        }
        for action in &pkg_remove {
            if let ApplyAction::RemovePackages { packages: pkgs } = action {
                for pkg in pkgs {
                    output.list_item_status(
                        &format!("{} {} [PRUNE]", output.colored("-", "\x1b[31m"), pkg),
                        StatusBadge::Error,
                    );
                }
            }
        }
    }

    if !dotfile_actions.is_empty() || !dotfile_remove.is_empty() {
        output.subheader("Dotfiles");
        for action in &dotfile_actions {
            match action {
                ApplyAction::CreateSymlink {
                    source,
                    target,
                    module_id,
                } => {
                    output.list_item_status(
                        &format!(
                            "{} {} -> {} ({})",
                            output.colored("+", "\x1b[32m"),
                            target,
                            source,
                            module_id
                        ),
                        StatusBadge::Ok,
                    );
                }
                ApplyAction::RenderAndCopy {
                    target, module_id, ..
                } => {
                    output.list_item_status(
                        &format!(
                            "{} {} (template, {})",
                            output.colored("~", "\x1b[33m"),
                            target,
                            module_id
                        ),
                        StatusBadge::Warning,
                    );
                }
                ApplyAction::CopyFile {
                    target, module_id, ..
                } => {
                    output.list_item_status(
                        &format!(
                            "{} {} (copy, {})",
                            output.colored("+", "\x1b[32m"),
                            target,
                            module_id
                        ),
                        StatusBadge::Ok,
                    );
                }
                _ => {}
            }
        }
        for action in &dotfile_remove {
            if let ApplyAction::RemoveSymlink { target } = action {
                output.list_item_status(
                    &format!("{} {} [PRUNE]", output.colored("-", "\x1b[31m"), target),
                    StatusBadge::Error,
                );
            }
        }
    }

    if !svc_enable.is_empty() || !svc_disable.is_empty() {
        output.subheader("Services");
        for action in &svc_enable {
            if let ApplyAction::EnableService { name } = action {
                output.list_item_status(
                    &format!("{} {}", output.colored("+", "\x1b[32m"), name),
                    StatusBadge::Ok,
                );
            }
        }
        for action in &svc_disable {
            if let ApplyAction::DisableService { name } = action {
                output.list_item_status(
                    &format!("{} {} [PRUNE]", output.colored("-", "\x1b[31m"), name),
                    StatusBadge::Error,
                );
            }
        }
    }

    if !mod_activate.is_empty() || !mod_deactivate.is_empty() {
        output.subheader("Modules");
        for action in &mod_activate {
            if let ApplyAction::ActivateModule { id } = action {
                output.list_item_status(
                    &format!("{} {}", output.colored("+", "\x1b[32m"), id),
                    StatusBadge::Ok,
                );
            }
        }
        for action in &mod_deactivate {
            if let ApplyAction::DeactivateModule { id } = action {
                output.list_item_status(
                    &format!("{} {} [PRUNE]", output.colored("-", "\x1b[31m"), id),
                    StatusBadge::Error,
                );
            }
        }
    }

    output.separator();
    output.info(&format!("Summary: {}", plan.summary()));
    output.info("Run `iron apply` to execute this plan.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use iron_core::services::apply::{ApplyAction, ApplyPlan};

    #[test]
    fn test_plan_groups_by_type() {
        let plan = ApplyPlan {
            actions: vec![
                ApplyAction::InstallPackages {
                    packages: vec!["neovim".into()],
                },
                ApplyAction::CreateSymlink {
                    source: "/src".into(),
                    target: "/tgt".into(),
                    module_id: "test".into(),
                },
                ApplyAction::EnableService {
                    name: "bluetooth".into(),
                },
                ApplyAction::ActivateModule { id: "test".into() },
            ],
        };
        assert_eq!(plan.action_count(), 4);
        assert!(!plan.is_empty());
    }

    #[test]
    fn test_empty_plan_display() {
        let plan = ApplyPlan::default();
        assert!(plan.is_empty());
        assert_eq!(plan.action_count(), 0);
    }
}
