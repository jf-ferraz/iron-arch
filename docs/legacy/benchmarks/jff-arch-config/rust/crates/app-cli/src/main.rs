use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use core_domain::{
    build_plan, get_active_host, get_active_theme, get_default_host, get_status_summary,
    list_hosts, load_modules, load_services, load_themes, set_active_host, set_active_theme,
    validate_repository, ServiceEntry,
};
use serde_json::json;

const EXIT_OK: i32 = 0;
const EXIT_RUNTIME_ERROR: i32 = 1;
const EXIT_USAGE_ERROR: i32 = 2;
const EXIT_VALIDATION_ERROR: i32 = 3;
const EXIT_NOT_FOUND: i32 = 4;
const EXIT_OPERATION_FAILED: i32 = 5;

fn main() {
    let started = Instant::now();
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let (args, root) = match extract_root_arg(&raw_args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("{msg}");
            print_usage();
            let _ = write_operation_log(
                &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                &raw_args,
                "unknown",
                "repository",
                2,
                started.elapsed().as_millis(),
            );
            process::exit(2);
        }
    };

    if args.is_empty() {
        print_usage();
        let _ = write_operation_log(
            &root,
            &raw_args,
            "unknown",
            "repository",
            2,
            started.elapsed().as_millis(),
        );
        process::exit(EXIT_USAGE_ERROR);
    }

    let operation_name = operation_name(&args);
    let module_name = module_name(&args);

    let exit_code = match args[0].as_str() {
        "validate" => run_validate(&root),
        "status" => run_status(&root),
        "doctor" => run_doctor(&root, &args[1..]),
        "quickstart" => run_quickstart(&root, &args[1..]),
        "go" => run_quickstart(&root, &args[1..]),
        "host" => run_host(&root, &args[1..]),
        "theme" => run_theme(&root, &args[1..]),
        "service" => run_service(&root, &args[1..]),
        "plan" => run_plan(&root, &args[1..]),
        "apply" => run_apply(&root, &args[1..]),
        "run" => run_task(&root, &args[1..]),
        "help" | "--help" | "-h" => {
            print_usage();
            EXIT_OK
        }
        other => {
            eprintln!("Unknown command: {other}");
            print_usage();
            EXIT_USAGE_ERROR
        }
    };

    let _ = write_operation_log(
        &root,
        &raw_args,
        operation_name,
        module_name,
        exit_code,
        started.elapsed().as_millis(),
    );

    process::exit(exit_code);
}

fn run_validate(root: &Path) -> i32 {
    match validate_repository(root) {
        Ok(report) => {
            println!("Validation root: {}", root.display());
            println!("Files checked: {}", report.files_checked);

            if report.issues.is_empty() {
                println!("Result: OK");
                return EXIT_OK;
            }

            for issue in &report.issues {
                println!(
                    "[{}] {}: {}",
                    issue.severity,
                    issue.path.display(),
                    issue.message
                );
            }

            println!(
                "Summary: {} error(s), {} warning(s)",
                report.error_count(),
                report.warning_count()
            );

            if report.ok() {
                EXIT_OK
            } else {
                EXIT_VALIDATION_ERROR
            }
        }
        Err(e) => {
            eprintln!("Validation failed: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

fn run_status(root: &Path) -> i32 {
    match get_status_summary(root) {
        Ok(summary) => {
            println!("Repository root: {}", summary.root.display());
            println!("Manifest files: {}", summary.manifest_count);
            println!("Module descriptors: {}", summary.module_count);
            println!(
                "Default host: {}",
                summary
                    .default_host
                    .as_deref()
                    .unwrap_or("<unset in app/manifests/hosts.toml>")
            );
            println!(
                "Active host: {}",
                summary
                    .active_host
                    .as_deref()
                    .unwrap_or("<unset in app/state/run/active-host>")
            );
            EXIT_OK
        }
        Err(e) => {
            eprintln!("Status failed: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

fn run_doctor(root: &Path, args: &[String]) -> i32 {
    let mut strict = false;
    let mut json_output = false;
    let mut apply = false;
    let mut category: Option<&str> = None;
    let mut use_script = false;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--strict" => {
                strict = true;
                i += 1;
            }
            "--json" => {
                json_output = true;
                use_script = true;
                i += 1;
            }
            "--apply" => {
                apply = true;
                use_script = true;
                i += 1;
            }
            "--dry-run" => {
                apply = false;
                i += 1;
            }
            arg if arg.starts_with("--category=") => {
                category = Some(arg.trim_start_matches("--category="));
                use_script = true;
                i += 1;
            }
            "--full" => {
                use_script = true;
                i += 1;
            }
            unknown => {
                eprintln!("unknown argument for doctor: {unknown}");
                return EXIT_USAGE_ERROR;
            }
        }
    }

    // If advanced options are used, delegate to the shell script
    if use_script {
        let mut script_args = Vec::new();
        if apply {
            script_args.push("--apply".to_string());
        } else {
            script_args.push("--dry-run".to_string());
        }
        if json_output {
            script_args.push("--json".to_string());
        }
        if strict {
            script_args.push("--strict".to_string());
        }
        if let Some(cat) = category {
            script_args.push(format!("--category={}", cat));
        }

        let cmd_str = format!("scripts/doctor-system.sh {}", script_args.join(" "));
        println!("Running: {cmd_str}");

        let status = Command::new("sh")
            .arg("-c")
            .arg(&cmd_str)
            .current_dir(root)
            .status();

        return match status {
            Ok(s) if s.success() => EXIT_OK,
            Ok(s) => s.code().unwrap_or(EXIT_OPERATION_FAILED),
            Err(e) => {
                eprintln!("failed to execute doctor-system.sh: {e}");
                EXIT_RUNTIME_ERROR
            }
        };
    }

    // Default: run inline repository checks (quick mode)
    let mut warnings = 0usize;
    let mut errors = 0usize;

    println!("System Doctor (Quick Mode)");
    println!("Root: {}", root.display());
    println!("Tip: Use --full for comprehensive system health checks");
    println!();

    match validate_repository(root) {
        Ok(report) => {
            if report.error_count() == 0 {
                println!(
                    "[OK] Manifest validation passed ({} files checked, {} warning(s)).",
                    report.files_checked,
                    report.warning_count()
                );
                warnings += report.warning_count();
            } else {
                println!(
                    "[ERROR] Manifest validation failed ({} error(s), {} warning(s)).",
                    report.error_count(),
                    report.warning_count()
                );
                errors += report.error_count();
                warnings += report.warning_count();
            }
        }
        Err(e) => {
            println!("[ERROR] Could not run validation: {e}");
            errors += 1;
        }
    }

    match get_status_summary(root) {
        Ok(summary) => {
            println!(
                "[OK] Repo status: {} manifests, {} modules, default host '{}', active host '{}'.",
                summary.manifest_count,
                summary.module_count,
                summary.default_host.as_deref().unwrap_or("<unset>"),
                summary.active_host.as_deref().unwrap_or("<unset>")
            );
        }
        Err(e) => {
            println!("[ERROR] Could not read repository status: {e}");
            errors += 1;
        }
    }

    for (name, hint) in [
        ("stow", "sudo pacman -S stow"),
        ("systemctl", "installed with systemd"),
        ("busctl", "installed with systemd"),
        ("cargo", "sudo pacman -S rust"),
    ] {
        if command_exists(name) {
            println!("[OK] Tool available: {name}");
        } else {
            println!("[WARN] Missing tool: {name} (hint: {hint})");
            warnings += 1;
        }
    }

    for rel in [
        "scripts/deploy-hypr.sh",
        "scripts/update-system.sh",
        "scripts/clean-system.sh",
        "scripts/doctor-system.sh",
        "scripts/check-secrets-service.sh",
        "app/manifests/hosts.toml",
        "app/manifests/services.toml",
        "app/manifests/update-policy.toml",
    ] {
        let p = root.join(rel);
        if p.exists() {
            println!("[OK] Found: {rel}");
        } else {
            println!("[ERROR] Missing required file: {rel}");
            errors += 1;
        }
    }

    match load_services(root) {
        Ok(services) => {
            println!("[OK] Loaded {} services from manifest.", services.len());
            let user_unit_dir = std::env::var("HOME")
                .map(|h| Path::new(&h).join(".config/systemd/user"))
                .ok();
            for svc in services {
                if svc.service_type == "systemd-user" {
                    let Some(unit) = svc.unit.as_deref() else {
                        println!("[ERROR] service '{}' has no unit", svc.id);
                        errors += 1;
                        continue;
                    };
                    if let Some(dir) = &user_unit_dir {
                        let unit_path = dir.join(unit);
                        if unit_path.exists() {
                            println!("[OK] User unit synced: {}", unit);
                        } else {
                            println!(
                                "[WARN] User unit not synced yet: {} (run: app-cli service sync-user-units --root .)",
                                unit
                            );
                            warnings += 1;
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("[ERROR] Failed to load services manifest: {e}");
            errors += 1;
        }
    }

    let log_path = root.join("app/state/logs/operations.jsonl");
    if log_path.exists() {
        println!("[OK] Operation log present: app/state/logs/operations.jsonl");
    } else {
        println!("[WARN] Operation log not found yet. It will be created after running CLI/script commands.");
        warnings += 1;
    }

    println!();
    println!(
        "Doctor Summary: {} error(s), {} warning(s)",
        errors, warnings
    );
    if errors > 0 {
        return EXIT_RUNTIME_ERROR;
    }
    if strict && warnings > 0 {
        return EXIT_VALIDATION_ERROR;
    }
    EXIT_OK
}

fn run_quickstart(root: &Path, args: &[String]) -> i32 {
    let mut apply = false;
    for arg in args {
        match arg.as_str() {
            "--apply" => apply = true,
            unknown => {
                eprintln!("unknown argument for quickstart: {unknown}");
                return EXIT_USAGE_ERROR;
            }
        }
    }

    println!("Quickstart Mode");
    println!("Root: {}", root.display());
    println!();

    let mut failures = 0usize;

    println!("Step 1/4: Doctor check");
    let doctor_code = run_doctor(root, &[]);
    if doctor_code != EXIT_OK {
        failures += 1;
        println!("Result: doctor reported issues (exit code {doctor_code}).");
    } else {
        println!("Result: doctor passed.");
    }
    println!();

    println!("Step 2/4: Hyprland plan preview");
    let plan_args = vec!["--module".to_string(), "hyprland".to_string()];
    let plan_code = run_plan(root, &plan_args);
    if plan_code != EXIT_OK {
        failures += 1;
        println!("Result: plan failed (exit code {plan_code}).");
    } else {
        println!("Result: plan generated.");
    }
    println!();

    if apply {
        println!("Step 3/4: Hyprland apply");
        let apply_args = vec!["hyprland".to_string()];
        let apply_code = run_apply(root, &apply_args);
        if apply_code != EXIT_OK {
            failures += 1;
            println!("Result: hyprland apply failed (exit code {apply_code}).");
        } else {
            println!("Result: hyprland apply completed.");
        }
        println!();

        println!("Step 4/4: Service defaults apply");
        let service_args = vec!["apply-defaults".to_string()];
        let service_code = run_service(root, &service_args);
        if service_code != EXIT_OK {
            failures += 1;
            println!("Result: service defaults apply failed (exit code {service_code}).");
        } else {
            println!("Result: service defaults applied.");
        }
        println!();
    } else {
        println!("Step 3/4: Hyprland apply (dry-run)");
        let apply_args = vec!["hyprland".to_string(), "--dry-run".to_string()];
        let apply_code = run_apply(root, &apply_args);
        if apply_code != EXIT_OK {
            failures += 1;
            println!("Result: hyprland dry-run failed (exit code {apply_code}).");
        } else {
            println!("Result: hyprland dry-run completed.");
        }
        println!();

        println!("Step 4/4: Service defaults apply (dry-run)");
        let service_args = vec!["apply-defaults".to_string(), "--dry-run".to_string()];
        let service_code = run_service(root, &service_args);
        if service_code != EXIT_OK {
            failures += 1;
            println!("Result: service defaults dry-run failed (exit code {service_code}).");
        } else {
            println!("Result: service defaults dry-run completed.");
        }
        println!();
    }

    if failures == 0 {
        if apply {
            println!("Quickstart Summary: SUCCESS");
            println!("All guided steps were applied successfully.");
        } else {
            println!("Quickstart Summary: SAFE TO APPLY");
            println!("All guided dry-runs passed. You can now run:");
            println!("  app-cli quickstart --apply --root {}", root.display());
        }
        EXIT_OK
    } else {
        println!(
            "Quickstart Summary: {} step(s) reported failures.",
            failures
        );
        EXIT_OPERATION_FAILED
    }
}

fn run_host(root: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("host command requires a subcommand: list | show | set <id>");
        return EXIT_USAGE_ERROR;
    }

    match args[0].as_str() {
        "list" => match list_hosts(root) {
            Ok(hosts) => {
                for host in hosts {
                    println!("{host}");
                }
                EXIT_OK
            }
            Err(e) => {
                eprintln!("host list failed: {e}");
                EXIT_RUNTIME_ERROR
            }
        },
        "show" => {
            let active = match get_active_host(root) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("host show failed: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            };
            let default_host = match get_default_host(root) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("host show failed: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            };

            println!("active={}", active.as_deref().unwrap_or("<unset>"));
            println!("default={}", default_host.as_deref().unwrap_or("<unset>"));
            EXIT_OK
        }
        "set" => {
            let Some(host_id) = args.get(1) else {
                eprintln!("host set requires <id>");
                return EXIT_USAGE_ERROR;
            };
            match set_active_host(root, host_id) {
                Ok(()) => {
                    println!("active host set to '{host_id}'");
                    EXIT_OK
                }
                Err(e) => {
                    eprintln!("host set failed: {e}");
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        unknown => {
            eprintln!("unknown host subcommand: {unknown}");
            EXIT_USAGE_ERROR
        }
    }
}

fn run_theme(root: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("theme command requires a subcommand: list | current | switch <id>");
        return EXIT_USAGE_ERROR;
    }

    match args[0].as_str() {
        "list" => match load_themes(root) {
            Ok(themes) => {
                if themes.is_empty() {
                    println!("No themes configured in app/manifests/themes.toml");
                } else {
                    println!("Available themes:");
                    for theme in themes {
                        let desc = theme
                            .description
                            .as_deref()
                            .unwrap_or("No description");
                        println!(
                            "  {} - {} (shell: {})",
                            theme.id, theme.name, theme.shell
                        );
                        println!("      {desc}");
                    }
                }
                EXIT_OK
            }
            Err(e) => {
                eprintln!("theme list failed: {e}");
                EXIT_RUNTIME_ERROR
            }
        },
        "current" => {
            match get_active_theme(root) {
                Ok(Some(theme_id)) => {
                    println!("Active theme: {theme_id}");
                    // Show theme details if available
                    if let Ok(themes) = load_themes(root) {
                        if let Some(theme) = themes.iter().find(|t| t.id == theme_id) {
                            println!("  Name: {}", theme.name);
                            println!("  Shell: {}", theme.shell);
                            if let Some(desc) = &theme.description {
                                println!("  Description: {desc}");
                            }
                        }
                    }
                    EXIT_OK
                }
                Ok(None) => {
                    println!("No theme currently active");
                    EXIT_OK
                }
                Err(e) => {
                    eprintln!("theme current failed: {e}");
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        "switch" => {
            let Some(theme_id) = args.get(1) else {
                eprintln!("theme switch requires <theme-id>");
                eprintln!("Run 'app-cli theme list --root .' to see available themes");
                return EXIT_USAGE_ERROR;
            };
            match set_active_theme(root, theme_id) {
                Ok(()) => {
                    println!("Theme switched to '{theme_id}'");
                    println!();
                    println!("Note: To fully apply this theme, you may need to:");
                    println!("  1. Restart your shell (or run: source ~/.zshrc)");
                    println!("  2. Deploy theme dotfiles if configured");
                    EXIT_OK
                }
                Err(e) => {
                    eprintln!("theme switch failed: {e}");
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        unknown => {
            eprintln!("unknown theme subcommand: {unknown}");
            EXIT_USAGE_ERROR
        }
    }
}

fn run_service(root: &Path, args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!(
            "service command requires a subcommand: list|sync-user-units|apply-defaults|status|enable|disable|start|stop"
        );
        return EXIT_USAGE_ERROR;
    }

    match args[0].as_str() {
        "sync-user-units" => {
            let status = Command::new("sh")
                .arg("-c")
                .arg("scripts/systemd-sync-user-units.sh")
                .current_dir(root)
                .status();
            match status {
                Ok(s) if s.success() => EXIT_OK,
                Ok(s) => {
                    let code = s.code().unwrap_or(-1);
                    eprintln!("sync-user-units failed with exit code {code}");
                    EXIT_OPERATION_FAILED
                }
                Err(e) => {
                    eprintln!("failed to execute sync-user-units: {e}");
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        "apply-defaults" => {
            let mut dry_run = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--dry-run" => dry_run = true,
                    unknown => {
                        eprintln!("unknown argument for service apply-defaults: {unknown}");
                        return EXIT_USAGE_ERROR;
                    }
                }
            }

            let services = match load_services(root) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("service apply-defaults failed loading manifest: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            };

            let managed: Vec<ServiceEntry> = services
                .into_iter()
                .filter(|s| s.enabled)
                .filter(|s| s.service_type == "systemd-user" || s.service_type == "systemd-system")
                .collect();

            if managed.is_empty() {
                println!("No enabled systemd-managed services found in services.toml.");
                return EXIT_OK;
            }

            println!("Applying default managed services (enabled=true):");
            for s in &managed {
                println!(
                    "- {} (type={}, unit={})",
                    s.id,
                    s.service_type,
                    s.unit.as_deref().unwrap_or("-")
                );
            }

            let has_user_services = managed.iter().any(|s| s.service_type == "systemd-user");
            if has_user_services {
                if dry_run {
                    println!("DRY-RUN: would run scripts/systemd-sync-user-units.sh");
                } else {
                    let sync_status = Command::new("sh")
                        .arg("-c")
                        .arg("scripts/systemd-sync-user-units.sh")
                        .current_dir(root)
                        .status();
                    match sync_status {
                        Ok(s) if s.success() => {}
                        Ok(s) => {
                            let code = s.code().unwrap_or(-1);
                            eprintln!("sync-user-units failed with exit code {code}");
                            return EXIT_OPERATION_FAILED;
                        }
                        Err(e) => {
                            eprintln!("failed to execute sync-user-units: {e}");
                            return EXIT_RUNTIME_ERROR;
                        }
                    }
                }
            }

            let mut failures = 0usize;
            let mut applied = 0usize;
            for svc in managed {
                let Some(unit) = svc.unit.clone() else {
                    eprintln!("service '{}' is enabled but has no unit", svc.id);
                    failures += 1;
                    continue;
                };
                let scope = if svc.service_type == "systemd-user" {
                    "user"
                } else {
                    "system"
                };

                for action in ["enable", "start"] {
                    if dry_run {
                        if scope == "user" {
                            println!("DRY-RUN: systemctl --user {action} {unit}");
                        } else {
                            println!("DRY-RUN: systemctl {action} {unit}");
                        }
                        continue;
                    }

                    let mut cmd = Command::new("systemctl");
                    if scope == "user" {
                        cmd.arg("--user");
                    }
                    let status = cmd.arg(action).arg(&unit).current_dir(root).status();
                    match status {
                        Ok(s) if s.success() => {
                            applied += 1;
                        }
                        Ok(s) => {
                            let code = s.code().unwrap_or(-1);
                            eprintln!(
                                "failed: {} {} {} (exit code {})",
                                if scope == "user" {
                                    "systemctl --user"
                                } else {
                                    "systemctl"
                                },
                                action,
                                unit,
                                code
                            );
                            failures += 1;
                        }
                        Err(e) => {
                            eprintln!(
                                "failed to execute {} {} {}: {}",
                                if scope == "user" {
                                    "systemctl --user"
                                } else {
                                    "systemctl"
                                },
                                action,
                                unit,
                                e
                            );
                            failures += 1;
                        }
                    }
                }
            }

            if dry_run {
                println!("DRY-RUN complete. No changes applied.");
                return EXIT_OK;
            }

            println!("Service defaults applied: {applied} successful systemctl actions.");
            if failures > 0 {
                eprintln!("Service defaults completed with {failures} failure(s).");
                EXIT_OPERATION_FAILED
            } else {
                EXIT_OK
            }
        }
        "list" => {
            let services = match load_services(root) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("service list failed: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            };

            for s in services {
                println!(
                    "{}\ttype={}\tunit={}\tenabled={}",
                    s.id,
                    s.service_type,
                    s.unit.as_deref().unwrap_or("-"),
                    s.enabled
                );
            }
            EXIT_OK
        }
        action @ ("status" | "enable" | "disable" | "start" | "stop") => {
            let Some(target) = args.get(1) else {
                eprintln!("service {action} requires <id|unit>");
                return EXIT_USAGE_ERROR;
            };

            let mut scope_override: Option<&str> = None;
            let mut i = 2usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--scope" => {
                        let Some(v) = args.get(i + 1) else {
                            eprintln!("--scope requires user|system");
                            return EXIT_USAGE_ERROR;
                        };
                        if v != "user" && v != "system" {
                            eprintln!("invalid --scope '{}', expected user|system", v);
                            return EXIT_USAGE_ERROR;
                        }
                        scope_override = Some(v);
                        i += 2;
                    }
                    unknown => {
                        eprintln!("unknown argument for service {action}: {unknown}");
                        return EXIT_USAGE_ERROR;
                    }
                }
            }

            let services = match load_services(root) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("service {action} failed loading manifest: {e}");
                    return EXIT_RUNTIME_ERROR;
                }
            };

            let (unit, scope) = match resolve_service_target(&services, target, scope_override) {
                Ok(v) => v,
                Err(msg) => {
                    eprintln!("{msg}");
                    return EXIT_NOT_FOUND;
                }
            };

            let mut cmd = Command::new("systemctl");
            if scope == "user" {
                cmd.arg("--user");
            }
            cmd.arg(action).arg(&unit).current_dir(root);

            let scope_flag = if scope == "user" { "--user " } else { "" };
            println!("Running: systemctl {scope_flag}{action} {unit}");

            match cmd.status() {
                Ok(s) if s.success() => EXIT_OK,
                Ok(s) => {
                    let code = s.code().unwrap_or(-1);
                    eprintln!("systemctl {action} failed with exit code {code}");
                    EXIT_OPERATION_FAILED
                }
                Err(e) => {
                    eprintln!("failed to execute systemctl: {e}");
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        unknown => {
            eprintln!("unknown service subcommand: {unknown}");
            EXIT_USAGE_ERROR
        }
    }
}

fn resolve_service_target(
    services: &[ServiceEntry],
    target: &str,
    scope_override: Option<&str>,
) -> Result<(String, String), String> {
    if let Some(svc) = services.iter().find(|s| s.id == target) {
        return match svc.service_type.as_str() {
            "systemd-user" => {
                let unit = svc
                    .unit
                    .clone()
                    .ok_or_else(|| format!("service '{target}' has no unit"))?;
                Ok((unit, scope_override.unwrap_or("user").to_string()))
            }
            "systemd-system" => {
                let unit = svc
                    .unit
                    .clone()
                    .ok_or_else(|| format!("service '{target}' has no unit"))?;
                Ok((unit, scope_override.unwrap_or("system").to_string()))
            }
            other => Err(format!(
                "service '{target}' is type '{other}' and is not systemctl-managed"
            )),
        };
    }

    Ok((
        target.to_string(),
        scope_override.unwrap_or("user").to_string(),
    ))
}

fn run_plan(root: &Path, args: &[String]) -> i32 {
    let mut module_filter: Option<&str> = None;
    let mut operation_filter: Option<&str> = None;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--module" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("--module requires a value");
                    return EXIT_USAGE_ERROR;
                };
                module_filter = Some(v);
                i += 2;
            }
            "--operation" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("--operation requires a value");
                    return EXIT_USAGE_ERROR;
                };
                operation_filter = Some(v);
                i += 2;
            }
            unknown => {
                eprintln!("unknown argument for plan: {unknown}");
                return EXIT_USAGE_ERROR;
            }
        }
    }

    match build_plan(root, module_filter, operation_filter) {
        Ok(plan) => {
            if plan.is_empty() {
                println!("No operations matched the requested filters.");
                return EXIT_NOT_FOUND;
            }
            println!("Execution plan:");
            for item in plan {
                println!(
                    "{}. [{}] {} -> {}",
                    item.order, item.module_id, item.operation_id, item.command
                );
            }
            EXIT_OK
        }
        Err(e) => {
            eprintln!("plan failed: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

fn run_apply(root: &Path, args: &[String]) -> i32 {
    let Some(module_id) = args.first() else {
        eprintln!("apply requires <module>");
        return EXIT_USAGE_ERROR;
    };

    let mut operation_arg: Option<&str> = None;
    let mut dry_run = false;
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--operation" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("--operation requires a value");
                    return EXIT_USAGE_ERROR;
                };
                operation_arg = Some(v);
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            unknown => {
                eprintln!("unknown argument for apply: {unknown}");
                return EXIT_USAGE_ERROR;
            }
        }
    }

    let modules = match load_modules(root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("apply failed while loading modules: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };
    let Some(module) = modules.into_iter().find(|m| m.id == *module_id) else {
        eprintln!("module not found: {module_id}");
        return EXIT_NOT_FOUND;
    };

    let selected_operation_id = if let Some(op) = operation_arg {
        op.to_string()
    } else if dry_run {
        if module.operations.iter().any(|o| o.id == "deploy-dry-run") {
            "deploy-dry-run".to_string()
        } else if module.operations.iter().any(|o| o.id == "dry-run") {
            "dry-run".to_string()
        } else if module.operations.iter().any(|o| o.id == "lint") {
            "lint".to_string()
        } else {
            match module.operations.first() {
                Some(op) => op.id.clone(),
                None => {
                    eprintln!("module has no operations: {module_id}");
                    return EXIT_NOT_FOUND;
                }
            }
        }
    } else if module.operations.iter().any(|o| o.id == "deploy") {
        "deploy".to_string()
    } else if module.operations.iter().any(|o| o.id == "run") {
        "run".to_string()
    } else if module.operations.iter().any(|o| o.id == "apply") {
        "apply".to_string()
    } else {
        match module.operations.first() {
            Some(op) => op.id.clone(),
            None => {
                eprintln!("module has no operations: {module_id}");
                return EXIT_NOT_FOUND;
            }
        }
    };

    let Some(operation) = module
        .operations
        .iter()
        .find(|o| o.id == selected_operation_id)
        .cloned()
    else {
        eprintln!("operation '{selected_operation_id}' not found in module '{module_id}'");
        return EXIT_NOT_FOUND;
    };

    println!(
        "Applying module '{}' operation '{}' (dry_run={})",
        module_id, operation.id, dry_run
    );
    println!("Command: {}", operation.command);

    let status = Command::new("sh")
        .arg("-c")
        .arg(&operation.command)
        .current_dir(root)
        .status();

    match status {
        Ok(s) if s.success() => EXIT_OK,
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            eprintln!("operation failed with exit code {code}");
            EXIT_OPERATION_FAILED
        }
        Err(e) => {
            eprintln!("failed to execute operation command: {e}");
            EXIT_RUNTIME_ERROR
        }
    }
}

fn run_task(root: &Path, args: &[String]) -> i32 {
    let Some(task_name) = args.first() else {
        eprintln!("run requires a task name: system-update|system-doctor|system-clean|system-status|switcher");
        return EXIT_USAGE_ERROR;
    };

    match task_name.as_str() {
        "system-update" => {
            let mut script_args = vec!["--dry-run".to_string()];
            for arg in &args[1..] {
                match arg.as_str() {
                    "--apply" => {
                        script_args.retain(|s| s != "--dry-run");
                        if !script_args.contains(&"--apply".to_string()) {
                            script_args.push("--apply".to_string());
                        }
                    }
                    "--non-interactive" => {
                        if !script_args.contains(&"--non-interactive".to_string()) {
                            script_args.push("--non-interactive".to_string());
                        }
                    }
                    "--dry-run" => {
                        script_args.retain(|s| s != "--apply");
                        if !script_args.contains(&"--dry-run".to_string()) {
                            script_args.push("--dry-run".to_string());
                        }
                    }
                    unknown => {
                        eprintln!("unknown argument for run system-update: {unknown}");
                        return EXIT_USAGE_ERROR;
                    }
                }
            }

            let cmd_str = format!("scripts/update-system.sh {}", script_args.join(" "));
            println!("Running: {cmd_str}");

            let status = Command::new("sh")
                .arg("-c")
                .arg(&cmd_str)
                .current_dir(root)
                .status();

            match status {
                Ok(s) if s.success() => EXIT_OK,
                Ok(s) => {
                    let code = s.code().unwrap_or(-1);
                    eprintln!("system-update failed with exit code {code}");
                    EXIT_OPERATION_FAILED
                }
                Err(e) => {
                    eprintln!("failed to execute system-update: {e}");
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        "system-doctor" => run_doctor(root, &args[1..]),
        "system-status" => run_rich_status(root),
        "system-clean" => {
            let mut script_args = vec!["--dry-run".to_string()];
            for arg in &args[1..] {
                match arg.as_str() {
                    "--apply" => {
                        script_args.retain(|s| s != "--dry-run");
                        if !script_args.contains(&"--apply".to_string()) {
                            script_args.push("--apply".to_string());
                        }
                    }
                    "--dry-run" => {
                        script_args.retain(|s| s != "--apply");
                        if !script_args.contains(&"--dry-run".to_string()) {
                            script_args.push("--dry-run".to_string());
                        }
                    }
                    "--aggressive" => {
                        if !script_args.contains(&"--aggressive".to_string()) {
                            script_args.push("--aggressive".to_string());
                        }
                    }
                    arg if arg.starts_with("--category=") => {
                        script_args.push(arg.to_string());
                    }
                    unknown => {
                        eprintln!("unknown argument for run system-clean: {unknown}");
                        return EXIT_USAGE_ERROR;
                    }
                }
            }

            let cmd_str = format!("scripts/clean-system.sh {}", script_args.join(" "));
            println!("Running: {cmd_str}");

            let status = Command::new("sh")
                .arg("-c")
                .arg(&cmd_str)
                .current_dir(root)
                .status();

            match status {
                Ok(s) if s.success() => EXIT_OK,
                Ok(s) => {
                    let code = s.code().unwrap_or(-1);
                    eprintln!("system-clean failed with exit code {code}");
                    EXIT_OPERATION_FAILED
                }
                Err(e) => {
                    eprintln!("failed to execute system-clean: {e}");
                    EXIT_RUNTIME_ERROR
                }
            }
        }
        "switcher" => run_switcher(root, &args[1..]),
        unknown => {
            eprintln!("unknown task: {unknown}");
            eprintln!("Available tasks: system-update, system-doctor, system-clean, system-status, switcher");
            EXIT_USAGE_ERROR
        }
    }
}

fn run_rich_status(root: &Path) -> i32 {
    // Basic repo status first
    let basic_code = run_status(root);
    if basic_code != EXIT_OK {
        return basic_code;
    }

    println!();
    println!("--- Extended Status ---");

    // Pending updates count
    let updates_output = Command::new("sh")
        .arg("-c")
        .arg("checkupdates 2>/dev/null | wc -l")
        .current_dir(root)
        .output();
    match updates_output {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("Pending updates: {count}");
        }
        _ => println!("Pending updates: <checkupdates not available>"),
    }

    // Failed systemd units
    let failed_output = Command::new("sh")
        .arg("-c")
        .arg("systemctl --user --failed --no-legend 2>/dev/null | wc -l")
        .output();
    match failed_output {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("Failed user units: {count}");
        }
        _ => println!("Failed user units: <unknown>"),
    }
    let failed_system = Command::new("sh")
        .arg("-c")
        .arg("systemctl --failed --no-legend 2>/dev/null | wc -l")
        .output();
    match failed_system {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("Failed system units: {count}");
        }
        _ => println!("Failed system units: <unknown>"),
    }

    // Last update timestamp from operation log
    let log_path = root.join("app/state/logs/operations.jsonl");
    if log_path.exists() {
        if let Ok(raw) = fs::read_to_string(&log_path) {
            let mut last_update: Option<String> = None;
            for line in raw.lines().rev() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    let module = v.get("module").and_then(|m| m.as_str()).unwrap_or("");
                    let operation = v.get("operation").and_then(|o| o.as_str()).unwrap_or("");
                    if module == "updates" || operation == "run.system-update" {
                        if let Some(ts) = v.get("timestamp_unix_ms").and_then(|t| t.as_u64()) {
                            let secs = ts / 1000;
                            last_update = Some(format!("unix epoch {secs} (see operations.jsonl)"));
                        }
                        break;
                    }
                }
            }
            println!(
                "Last update: {}",
                last_update.as_deref().unwrap_or("<no update logged>")
            );
        }
    } else {
        println!("Last update: <no operation log>");
    }

    // Orphan package count
    let orphans = Command::new("sh")
        .arg("-c")
        .arg("pacman -Qtdq 2>/dev/null | wc -l")
        .output();
    match orphans {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("Orphan packages: {count}");
        }
        _ => println!("Orphan packages: <unknown>"),
    }

    // Package cache size
    let cache_size = Command::new("sh")
        .arg("-c")
        .arg("du -sh /var/cache/pacman/pkg 2>/dev/null | cut -f1")
        .output();
    match cache_size {
        Ok(out) if out.status.success() => {
            let size = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !size.is_empty() {
                println!("Package cache: {size}");
            } else {
                println!("Package cache: <unknown>");
            }
        }
        _ => println!("Package cache: <unknown>"),
    }

    // Stow symlink integrity (check if hyprland config is deployed)
    let home = env::var("HOME").unwrap_or_default();
    if !home.is_empty() {
        let hypr_conf = Path::new(&home).join(".config/hypr/hyprland.conf");
        if hypr_conf.is_symlink() {
            println!("Hyprland stow: deployed (symlink intact)");
        } else if hypr_conf.exists() {
            println!("Hyprland stow: file exists but is not a symlink");
        } else {
            println!("Hyprland stow: not deployed");
        }
    }

    EXIT_OK
}

fn run_switcher(root: &Path, args: &[String]) -> i32 {
    let Some(profile_name) = args.first() else {
        eprintln!("switcher requires a profile name");
        eprintln!("Usage: app-cli run switcher <profile-name>");

        // List available profiles
        let profiles_path = root.join("app/manifests/profiles.toml");
        if profiles_path.exists() {
            if let Ok(raw) = fs::read_to_string(&profiles_path) {
                if let Ok(value) = raw.parse::<toml::Value>() {
                    if let Some(profiles) = value.get("profiles").and_then(|v| v.as_array()) {
                        eprintln!("Available profiles:");
                        for p in profiles {
                            if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
                                let desc = p
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                eprintln!("  {id} - {desc}");
                            }
                        }
                    }
                }
            }
        }
        return EXIT_USAGE_ERROR;
    };

    let profiles_path = root.join("app/manifests/profiles.toml");
    if !profiles_path.exists() {
        eprintln!("Profiles manifest not found: app/manifests/profiles.toml");
        return EXIT_NOT_FOUND;
    }

    let raw = match fs::read_to_string(&profiles_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to read profiles manifest: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };

    let value: toml::Value = match raw.parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse profiles manifest: {e}");
            return EXIT_RUNTIME_ERROR;
        }
    };

    let profiles = match value.get("profiles").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            eprintln!("No [[profiles]] found in profiles.toml");
            return EXIT_RUNTIME_ERROR;
        }
    };

    let target_profile = profiles.iter().find(|p| {
        p.get("id")
            .and_then(|v| v.as_str())
            .map(|id| id == profile_name.as_str())
            .unwrap_or(false)
    });

    let Some(profile) = target_profile else {
        eprintln!("Profile not found: {profile_name}");
        eprintln!("Available profiles:");
        for p in profiles {
            if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
                eprintln!("  {id}");
            }
        }
        return EXIT_NOT_FOUND;
    };

    let enable_configs: Vec<&str> = profile
        .get("configs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();

    println!("Switching to profile: {profile_name}");
    println!("Enabled configs: {}", enable_configs.join(", "));

    // Load all config modules to find which to unstow
    let configs_path = root.join("app/manifests/configs.toml");
    let all_config_ids: Vec<String> = if configs_path.exists() {
        if let Ok(raw) = fs::read_to_string(&configs_path) {
            if let Ok(val) = raw.parse::<toml::Value>() {
                val.get("configs")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(String::from))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let mut failures = 0usize;

    // Unstow configs not in the target profile
    for config_id in &all_config_ids {
        if enable_configs.contains(&config_id.as_str()) {
            continue;
        }
        let stow_dir = root.join(format!("modules/configs/{config_id}/stow"));
        if !stow_dir.is_dir() {
            continue;
        }

        println!("Deactivating: {config_id}");
        let home = env::var("HOME").unwrap_or_default();
        let status = Command::new("stow")
            .arg("-D")
            .arg("-d")
            .arg(format!("modules/configs/{config_id}"))
            .arg("-t")
            .arg(&home)
            .arg("stow")
            .current_dir(root)
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                eprintln!("Warning: unstow {config_id} exited with code {code}");
                failures += 1;
            }
            Err(e) => {
                eprintln!("Warning: failed to unstow {config_id}: {e}");
                failures += 1;
            }
        }
    }

    // Stow configs in the target profile
    for config_id in &enable_configs {
        let stow_dir = root.join(format!("modules/configs/{config_id}/stow"));
        if !stow_dir.is_dir() {
            println!("Skipping {config_id}: no stow package found");
            continue;
        }

        println!("Activating: {config_id}");

        // Run host overlay if applicable
        let overlay_script = root.join(format!("scripts/hypr-sync-host-overlay.sh"));
        if *config_id == "hyprland" && overlay_script.exists() {
            let _ = Command::new("sh")
                .arg("-c")
                .arg("scripts/hypr-sync-host-overlay.sh")
                .current_dir(root)
                .status();
        }

        let home = env::var("HOME").unwrap_or_default();
        let status = Command::new("stow")
            .arg("-d")
            .arg(format!("modules/configs/{config_id}"))
            .arg("-t")
            .arg(&home)
            .arg("stow")
            .current_dir(root)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("  Activated {config_id} successfully.");
            }
            Ok(s) => {
                let code = s.code().unwrap_or(-1);
                eprintln!("  Failed to stow {config_id} (exit {code})");
                failures += 1;
            }
            Err(e) => {
                eprintln!("  Failed to stow {config_id}: {e}");
                failures += 1;
            }
        }
    }

    // Write active profile to state
    let state_dir = root.join("app/state/run");
    let _ = fs::create_dir_all(&state_dir);
    let _ = fs::write(state_dir.join("active-profile"), format!("{profile_name}\n"));

    if failures > 0 {
        eprintln!(
            "Profile switch completed with {failures} warning(s). Some configs may need manual attention."
        );
        EXIT_OPERATION_FAILED
    } else {
        println!("Profile '{profile_name}' activated successfully.");
        EXIT_OK
    }
}

fn extract_root_arg(args: &[String]) -> Result<(Vec<String>, PathBuf), String> {
    let mut root = env::current_dir().map_err(|e| format!("failed to read current dir: {e}"))?;
    let mut clean_args = Vec::new();

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => {
                let Some(path) = args.get(i + 1) else {
                    return Err("missing value for --root".to_string());
                };
                root = PathBuf::from(path);
                i += 2;
            }
            "--root=" => {
                return Err("use --root <path>, not --root=".to_string());
            }
            arg if arg.starts_with("--root=") => {
                root = PathBuf::from(arg.trim_start_matches("--root="));
                i += 1;
            }
            arg => {
                clean_args.push(arg.to_string());
                i += 1;
            }
        }
    }

    Ok((clean_args, root))
}

fn print_usage() {
    println!("jff app-cli");
    println!();
    println!("Usage:");
    println!("  app-cli <command> [args] [--root <path>]");
    println!();
    println!("Commands:");
    println!("  status      Print repository summary");
    println!("  validate    Validate manifests and module descriptors");
    println!("  doctor      Beginner-friendly health check with guidance");
    println!("              [--strict] [--full] [--json] [--apply] [--category=X]");
    println!("  quickstart [--apply]   Guided end-to-end setup flow");
    println!("  go [--apply]   Alias for quickstart");
    println!("  host list   List host ids from hosts manifest");
    println!("  host show   Show active/default host");
    println!("  host set <id>   Set active host in app/state/run/active-host");
    println!("  theme list  List available shell themes");
    println!("  theme current   Show currently active theme");
    println!("  theme switch <id>   Switch to specified theme");
    println!("  service list");
    println!("  service sync-user-units");
    println!("  service apply-defaults [--dry-run]");
    println!("  service status <id|unit> [--scope user|system]");
    println!("  service enable|disable|start|stop <id|unit> [--scope user|system]");
    println!("  plan [--module <id>] [--operation <id>]   Show ordered execution plan");
    println!("  apply <module> [--operation <id>] [--dry-run]   Execute module operation");
    println!("  run <task>  Run a system task:");
    println!("    system-update [--apply|--dry-run] [--non-interactive]");
    println!("    system-doctor [--strict] [--full] [--json] [--apply] [--category=X]");
    println!("    system-clean [--apply|--dry-run] [--aggressive] [--category=X]");
    println!("    system-status");
    println!("    switcher <profile-name>");
    println!("  help        Show this help");
    println!();
    println!("Doctor categories: packages, storage, services, stability, performance, security");
    println!("Clean categories: packages, journal, user-cache, thumbnails, logs, memory, browser, dev");
    println!();
    println!("Exit codes:");
    println!("  0 success");
    println!("  1 runtime failure");
    println!("  2 usage/argument error");
    println!("  3 validation failure");
    println!("  4 module/operation not found");
    println!("  5 operation command failed");
}

fn write_operation_log(
    root: &Path,
    raw_args: &[String],
    operation: &str,
    module: &str,
    exit_code: i32,
    duration_ms: u128,
) -> std::io::Result<()> {
    let log_dir = root.join("app/state/logs");
    fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("operations.jsonl");

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let entry = json!({
        "timestamp_unix_ms": timestamp_ms,
        "module": module,
        "operation": operation,
        "args": raw_args,
        "duration_ms": duration_ms,
        "result": if exit_code == 0 { "ok" } else { "error" },
        "exit_code": exit_code
    });

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{entry}")?;
    Ok(())
}

fn operation_name(args: &[String]) -> &str {
    if args.is_empty() {
        return "unknown";
    }
    match args[0].as_str() {
        "host" => {
            if args.len() > 1 {
                match args[1].as_str() {
                    "list" => "host.list",
                    "show" => "host.show",
                    "set" => "host.set",
                    _ => "host.unknown",
                }
            } else {
                "host.unknown"
            }
        }
        "validate" => "validate",
        "status" => "status",
        "doctor" => "doctor",
        "quickstart" => "quickstart",
        "go" => "go",
        "help" | "--help" | "-h" => "help",
        "service" => {
            if args.len() > 1 {
                match args[1].as_str() {
                    "list" => "service.list",
                    "sync-user-units" => "service.sync-user-units",
                    "apply-defaults" => "service.apply-defaults",
                    "status" => "service.status",
                    "enable" => "service.enable",
                    "disable" => "service.disable",
                    "start" => "service.start",
                    "stop" => "service.stop",
                    _ => "service.unknown",
                }
            } else {
                "service.unknown"
            }
        }
        "plan" => "plan",
        "apply" => "apply",
        "run" => {
            if args.len() > 1 {
                match args[1].as_str() {
                    "system-update" => "run.system-update",
                    "system-doctor" => "run.system-doctor",
                    "system-clean" => "run.system-clean",
                    "system-status" => "run.system-status",
                    "switcher" => "run.switcher",
                    _ => "run.unknown",
                }
            } else {
                "run.unknown"
            }
        }
        _ => "unknown",
    }
}

fn module_name(args: &[String]) -> &str {
    match args.first().map(String::as_str) {
        Some("host") => "hosts",
        Some("service") => "services",
        Some("apply") => args.get(1).map(String::as_str).unwrap_or("repository"),
        Some("run") => match args.get(1).map(String::as_str) {
            Some("system-update") => "updates",
            Some("system-doctor") => "repository",
            Some("system-clean") => "clean",
            Some("system-status") => "repository",
            Some("switcher") => "configs",
            _ => "repository",
        },
        _ => "repository",
    }
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", command))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
