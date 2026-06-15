//! Install Commands
//!
//! Reviewable Arch installation planning.

use crate::cli::InstallAction;
use crate::context::AppContext;
use anyhow::Result;
use iron_core::InstallPlan;
use iron_core::services::host::HostService;

/// Execute install command.
pub fn execute(ctx: &AppContext, action: InstallAction) -> Result<()> {
    match action {
        InstallAction::Plan {
            host,
            target,
            emit_script,
        } => plan(ctx, &host, &target, emit_script),
    }
}

fn plan(ctx: &AppContext, host_id: &str, target: &str, emit_script: bool) -> Result<()> {
    let output = &ctx.output;
    let host_service = ctx.host_service();
    let host = host_service.load_host(host_id)?;
    let plan = InstallPlan::from_host(&host, target)?;

    if emit_script {
        output.raw(&plan.to_shell_script());
        return Ok(());
    }

    if output.is_json() {
        output.json(&plan);
        return Ok(());
    }

    output.header(&format!("Install Plan: {}", plan.host_id));
    output.kv("Host", format!("{} ({})", plan.host_name, plan.host_id));
    output.kv("Target Mount", &plan.target_mount);

    if !plan.warnings.is_empty() {
        output.subheader("Warnings");
        for warning in &plan.warnings {
            output.warning(warning);
        }
    }

    output.subheader("Phases");
    for phase in &plan.phases {
        output.list_item(&format!("{} [{} steps]", phase.name, phase.steps.len()));
        if output.is_verbose() {
            for step in &phase.steps {
                let mode = if step.destructive {
                    "destructive"
                } else {
                    "safe"
                };
                output.verbose(&format!("{} ({})", step.description, mode));
                if let Some(command) = &step.command {
                    output.verbose(&format!("  {}", command.join(" ")));
                }
            }
        }
    }

    output.info("This is a dry plan only; no install commands were executed.");
    output.info("Use --emit-script to generate a reviewable shell script.");

    Ok(())
}
