use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use core_domain::{
    apply_toggles, format_time_ago, get_activation_stats, get_active_theme,
    get_all_modules_status, get_maintenance_state, get_maintenance_statuses, get_status_summary,
    load_modules, load_services, load_themes, prepare_toggles, time_since_last,
    validate_repository, MaintenanceStatus, ModuleDescriptor,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use serde_json::Value;

// ── Tab-based navigation ──────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tab {
    System,
    Modules,
    Themes,
    Services,
    Quickstart,
    Scripts,
}

impl Tab {
    fn all() -> &'static [Tab] {
        &[
            Tab::System,
            Tab::Modules,
            Tab::Themes,
            Tab::Services,
            Tab::Quickstart,
            Tab::Scripts,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            Tab::System => "System",
            Tab::Modules => "Modules",
            Tab::Themes => "Themes",
            Tab::Services => "Services",
            Tab::Quickstart => "Quickstart",
            Tab::Scripts => "Scripts",
        }
    }

    fn index(&self) -> usize {
        match self {
            Tab::System => 0,
            Tab::Modules => 1,
            Tab::Themes => 2,
            Tab::Services => 3,
            Tab::Quickstart => 4,
            Tab::Scripts => 5,
        }
    }

    fn from_index(idx: usize) -> Option<Tab> {
        match idx {
            0 => Some(Tab::System),
            1 => Some(Tab::Modules),
            2 => Some(Tab::Themes),
            3 => Some(Tab::Services),
            4 => Some(Tab::Quickstart),
            5 => Some(Tab::Scripts),
            _ => None,
        }
    }
}

// ── Simplified action model ───────────────────────────────

#[derive(Debug, Clone)]
struct SimpleAction {
    label: String,
    description: String,
    command: String,
    needs_confirm: bool,
    is_preview: bool, // True = safe dry-run
}

#[derive(Debug, Clone, Default)]
struct TabState {
    selected: usize,
    #[allow(dead_code)]
    scroll: usize,
}

impl TabState {
    fn clamp_selection(&mut self, max: usize) {
        if max > 0 && self.selected >= max {
            self.selected = max - 1;
        }
    }
}

// ── Operation log entry ───────────────────────────────────

#[derive(Debug, Clone)]
struct OpLogEntry {
    timestamp_ms: u64,
    module: String,
    #[allow(dead_code)]
    operation: String,
    result: String,
    #[allow(dead_code)]
    exit_code: i64,
    duration_ms: u64,
}

struct RunningTask {
    label: String,
    child: Child,
    started: Instant,
    spinner_frame: usize,
}

// ── App state ─────────────────────────────────────────────

struct App {
    root: PathBuf,
    validation_errors: usize,
    validation_warnings: usize,
    manifests: usize,
    modules: usize,
    services: usize,
    default_host: String,
    active_host: String,
    recent_ops: Vec<OpLogEntry>,
    load_error: Option<String>,
    status_line: String,

    // Tab-based navigation
    current_tab: Tab,
    tab_states: [TabState; 6],
    system_actions: Vec<SimpleAction>,
    module_actions: Vec<SimpleAction>,
    theme_actions: Vec<SimpleAction>,
    service_actions: Vec<SimpleAction>,
    quickstart_actions: Vec<SimpleAction>,
    script_actions: Vec<SimpleAction>,
    active_theme: String,

    // Module toggle state
    module_checkboxes: Vec<(ModuleDescriptor, bool)>, // (module, is_checked)
    active_module_count: usize,
    total_module_count: usize,

    confirm_pending: Option<usize>,
    running: Option<RunningTask>,
}

// ── Entry point ───────────────────────────────────────────

fn main() -> io::Result<()> {
    let root = parse_root_arg()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &root);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn parse_root_arg() -> io::Result<PathBuf> {
    let mut root = env::current_dir()?;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "--root" => {
                let Some(path) = args.get(i + 1) else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--root requires a path",
                    ));
                };
                root = PathBuf::from(path);
                i += 2;
            }
            arg if arg.starts_with("--root=") => {
                root = PathBuf::from(arg.trim_start_matches("--root="));
                i += 1;
            }
            "-h" | "--help" => {
                println!("Usage: app-tui [--root <path>]");
                std::process::exit(0);
            }
            unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {unknown}"),
                ));
            }
        }
    }

    Ok(root)
}

// ── Event loop ────────────────────────────────────────────

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, root: &Path) -> io::Result<()> {
    let mut app = App::new(root);
    let tick_rate = Duration::from_millis(100);
    let refresh_rate = Duration::from_secs(2);
    let mut last_tick = Instant::now();
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|f| draw_ui(f, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(quit) = app.handle_key(key) {
                        if quit {
                            return Ok(());
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.poll_running();
            last_tick = Instant::now();
        }

        if app.running.is_none() && last_refresh.elapsed() >= refresh_rate {
            app.refresh_recent_logs();
            last_refresh = Instant::now();
        }
    }
}

// ── App implementation ────────────────────────────────────

impl App {
    fn new(root: &Path) -> Self {
        let mut app = App {
            root: root.to_path_buf(),
            validation_errors: 0,
            validation_warnings: 0,
            manifests: 0,
            modules: 0,
            services: 0,
            default_host: "<unknown>".to_string(),
            active_host: "<unknown>".to_string(),
            recent_ops: Vec::new(),
            load_error: None,
            status_line: "Ready".to_string(),

            current_tab: Tab::System,
            tab_states: Default::default(),
            system_actions: Vec::new(),
            module_actions: Vec::new(),
            theme_actions: Vec::new(),
            service_actions: Vec::new(),
            quickstart_actions: Vec::new(),
            script_actions: Vec::new(),
            active_theme: "<unknown>".to_string(),

            module_checkboxes: Vec::new(),
            active_module_count: 0,
            total_module_count: 0,

            confirm_pending: None,
            running: None,
        };
        app.refresh_all();
        app
    }

    fn refresh_all(&mut self) {
        self.load_error = None;

        match validate_repository(&self.root) {
            Ok(report) => {
                self.validation_errors = report.error_count();
                self.validation_warnings = report.warning_count();
            }
            Err(e) => self.load_error = Some(format!("validate failed: {e}")),
        }

        match get_status_summary(&self.root) {
            Ok(status) => {
                self.manifests = status.manifest_count;
                self.modules = status.module_count;
                self.default_host = status.default_host.unwrap_or_else(|| "<unset>".to_string());
                self.active_host = status.active_host.unwrap_or_else(|| "<unset>".to_string());
            }
            Err(e) => self.load_error = Some(format!("status failed: {e}")),
        }

        match load_services(&self.root) {
            Ok(services) => self.services = services.len(),
            Err(e) => self.load_error = Some(format!("services failed: {e}")),
        }

        match load_modules(&self.root) {
            Ok(mods) => self.modules = mods.len(),
            Err(e) => self.load_error = Some(format!("module load failed: {e}")),
        }

        match get_active_theme(&self.root) {
            Ok(Some(theme)) => self.active_theme = theme,
            Ok(None) => self.active_theme = "<none>".to_string(),
            Err(_) => self.active_theme = "<error>".to_string(),
        }

        match get_all_modules_status(&self.root) {
            Ok(modules_status) => self.module_checkboxes = modules_status,
            Err(_) => self.module_checkboxes = Vec::new(),
        }

        match get_activation_stats(&self.root) {
            Ok((active, total)) => {
                self.active_module_count = active;
                self.total_module_count = total;
            }
            Err(_) => {
                self.active_module_count = 0;
                self.total_module_count = 0;
            }
        }

        self.refresh_recent_logs();
        self.rebuild_tab_actions();
    }

    fn rebuild_tab_actions(&mut self) {
        self.system_actions = build_system_actions();
        self.module_actions = build_module_actions(&self.root);
        self.theme_actions = build_theme_actions(&self.root);
        self.service_actions = build_service_actions(&self.root);
        self.quickstart_actions = build_quickstart_actions();
        self.script_actions = build_script_actions();

        // Clamp selections after rebuilding
        for (idx, actions) in [
            &self.system_actions,
            &self.module_actions,
            &self.theme_actions,
            &self.service_actions,
            &self.quickstart_actions,
            &self.script_actions,
        ]
        .iter()
        .enumerate()
        {
            self.tab_states[idx].clamp_selection(actions.len());
        }
    }

    fn refresh_recent_logs(&mut self) {
        self.recent_ops = read_recent_ops(&self.root, 200);
    }

    fn current_actions(&self) -> &[SimpleAction] {
        match self.current_tab {
            Tab::System => &self.system_actions,
            Tab::Modules => &self.module_actions,
            Tab::Themes => &self.theme_actions,
            Tab::Services => &self.service_actions,
            Tab::Quickstart => &self.quickstart_actions,
            Tab::Scripts => &self.script_actions,
        }
    }

    fn current_tab_state(&self) -> &TabState {
        &self.tab_states[self.current_tab.index()]
    }

    fn current_tab_state_mut(&mut self) -> &mut TabState {
        &mut self.tab_states[self.current_tab.index()]
    }

    fn current_action(&self) -> Option<&SimpleAction> {
        let actions = self.current_actions();
        let state = self.current_tab_state();
        actions.get(state.selected)
    }

    // ── Key handling ──────────────────────────────────────

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Option<bool> {
        let code = key.code;

        // Running state: only 'q' kills and quits
        if self.running.is_some() {
            if let KeyCode::Char('q') = code {
                if let Some(mut task) = self.running.take() {
                    let _ = task.child.kill();
                    let _ = task.child.wait();
                }
                return Some(true);
            }
            return None;
        }

        // Confirm dialog
        if self.confirm_pending.is_some() {
            match code {
                KeyCode::Char('y') => {
                    let idx = self.confirm_pending.take().unwrap_or(0);
                    // Special case for module toggles
                    if idx == usize::MAX {
                        self.apply_module_changes();
                    } else {
                        self.spawn_action(idx);
                    }
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.confirm_pending = None;
                    self.status_line = "Action cancelled.".to_string();
                }
                _ => {}
            }
            return None;
        }

        // Normal mode
        match code {
            KeyCode::Char('q') => Some(true),
            KeyCode::Char('r') => {
                self.refresh_all();
                self.status_line = "Refreshed.".to_string();
                None
            }
            // Tab switching with arrow keys
            KeyCode::Left => {
                let idx = self.current_tab.index();
                if idx > 0 {
                    self.current_tab = Tab::from_index(idx - 1).unwrap_or(Tab::System);
                }
                None
            }
            KeyCode::Right => {
                let idx = self.current_tab.index();
                if idx < 5 {
                    self.current_tab = Tab::from_index(idx + 1).unwrap_or(Tab::Scripts);
                }
                None
            }
            // Number keys for direct tab jumping
            KeyCode::Char('1') => {
                self.current_tab = Tab::System;
                None
            }
            KeyCode::Char('2') => {
                self.current_tab = Tab::Modules;
                None
            }
            KeyCode::Char('3') => {
                self.current_tab = Tab::Themes;
                None
            }
            KeyCode::Char('4') => {
                self.current_tab = Tab::Services;
                None
            }
            KeyCode::Char('5') => {
                self.current_tab = Tab::Quickstart;
                None
            }
            KeyCode::Char('6') => {
                self.current_tab = Tab::Scripts;
                None
            }
            // Item navigation within current tab
            KeyCode::Up => {
                let state = self.current_tab_state_mut();
                if state.selected > 0 {
                    state.selected -= 1;
                }
                None
            }
            KeyCode::Down => {
                let max = self.current_actions().len();
                let state = self.current_tab_state_mut();
                if max > 0 && state.selected + 1 < max {
                    state.selected += 1;
                }
                None
            }
            KeyCode::PageUp => {
                let state = self.current_tab_state_mut();
                state.selected = state.selected.saturating_sub(10);
                None
            }
            KeyCode::PageDown => {
                let max = self.current_actions().len();
                let state = self.current_tab_state_mut();
                if max > 0 {
                    state.selected = (state.selected + 10).min(max - 1);
                }
                None
            }
            KeyCode::Home => {
                let state = self.current_tab_state_mut();
                state.selected = 0;
                None
            }
            KeyCode::End => {
                let max = self.current_actions().len();
                let state = self.current_tab_state_mut();
                if max > 0 {
                    state.selected = max - 1;
                }
                None
            }
            KeyCode::Char(' ') if self.current_tab == Tab::Modules => {
                // Toggle checkbox in Modules tab
                let sel = self.current_tab_state().selected;
                if let Some((_, is_active)) = self.module_checkboxes.get_mut(sel) {
                    *is_active = !*is_active;
                    self.status_line = "Space:toggle Enter:apply changes".to_string();
                }
                None
            }
            KeyCode::Enter => {
                // Shift+Enter on System tab Clean actions triggers aggressive mode
                if key.modifiers.contains(KeyModifiers::SHIFT) && self.current_tab == Tab::System {
                    let sel = self.current_tab_state().selected;
                    // Clean (preview) = index 4, Clean (apply) = index 5
                    if sel == 4 || sel == 5 {
                        if let Some(action) = self.current_actions().get(sel) {
                            let aggressive_cmd = action.command.replace("--dry-run", "--dry-run --aggressive")
                                .replace("--apply", "--apply --aggressive");
                            let label = format!("{} (aggressive)", action.label);
                            self.spawn_command(&label, &aggressive_cmd);
                            return None;
                        }
                    }
                }

                // Special handling for Modules tab - apply changes
                if self.current_tab == Tab::Modules {
                    let desired_active: Vec<String> = self
                        .module_checkboxes
                        .iter()
                        .filter_map(|(module, is_active)| {
                            if *is_active {
                                Some(module.id.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    match prepare_toggles(&self.root, &desired_active) {
                        Ok(toggles) if toggles.is_empty() => {
                            self.status_line = "No changes to apply".to_string();
                        }
                        Ok(toggles) => {
                            let changes: Vec<String> = toggles
                                .iter()
                                .map(|t| {
                                    if t.will_be_active {
                                        format!("Activate {}", t.module_id)
                                    } else {
                                        format!("Deactivate {}", t.module_id)
                                    }
                                })
                                .collect();
                            self.status_line = format!(
                                "Apply {} change(s)? Press 'y' to confirm or 'n' to cancel.",
                                changes.len()
                            );
                            // Store toggles in confirm_pending (using a special marker)
                            self.confirm_pending = Some(usize::MAX);
                        }
                        Err(e) => {
                            self.status_line = format!("Failed to prepare changes: {e}");
                        }
                    }
                    return None;
                }

                // Normal action execution for other tabs
                let sel = self.current_tab_state().selected;
                if let Some(action) = self.current_actions().get(sel) {
                    let needs_confirm = action.needs_confirm;
                    let label = action.label.clone();
                    if needs_confirm {
                        self.confirm_pending = Some(sel);
                        self.status_line = format!(
                            "Confirm '{}' with 'y' (or 'n' to cancel).",
                            label
                        );
                    } else {
                        self.spawn_action(sel);
                    }
                }
                None
            }
            _ => None,
        }
    }

    // ── Action execution ──────────────────────────────────

    fn spawn_action(&mut self, sel_idx: usize) {
        let Some(action) = self.current_actions().get(sel_idx).cloned() else {
            self.status_line = "Invalid action selection.".to_string();
            return;
        };

        let child = Command::new("sh")
            .arg("-c")
            .arg(&action.command)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(child) => {
                self.running = Some(RunningTask {
                    label: action.label.clone(),
                    child,
                    started: Instant::now(),
                    spinner_frame: 0,
                });
                self.status_line = format!("Running '{}'...", action.label);
            }
            Err(e) => {
                self.status_line = format!("Failed to start '{}': {}", action.label, e);
            }
        }
        self.confirm_pending = None;
    }

    fn spawn_command(&mut self, label: &str, command: &str) {
        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(child) => {
                self.running = Some(RunningTask {
                    label: label.to_string(),
                    child,
                    started: Instant::now(),
                    spinner_frame: 0,
                });
                self.status_line = format!("Running '{}'...", label);
            }
            Err(e) => {
                self.status_line = format!("Failed to start '{}': {}", label, e);
            }
        }
        self.confirm_pending = None;
    }

    fn apply_module_changes(&mut self) {
        let desired_active: Vec<String> = self
            .module_checkboxes
            .iter()
            .filter_map(|(module, is_active)| {
                if *is_active {
                    Some(module.id.clone())
                } else {
                    None
                }
            })
            .collect();

        match prepare_toggles(&self.root, &desired_active) {
            Ok(toggles) => match apply_toggles(&self.root, &toggles) {
                Ok(results) => {
                    let successful = results.iter().filter(|(_, ok)| *ok).count();
                    let failed = results.len() - successful;
                    if failed == 0 {
                        self.status_line = format!("Applied {} change(s) successfully", successful);
                    } else {
                        self.status_line = format!(
                            "Applied {} change(s), {} failed",
                            successful, failed
                        );
                    }
                    self.refresh_all();
                }
                Err(e) => {
                    self.status_line = format!("Failed to apply changes: {e}");
                }
            },
            Err(e) => {
                self.status_line = format!("Failed to prepare changes: {e}");
            }
        }
    }

    fn poll_running(&mut self) {
        if self.running.is_none() {
            return;
        }

        self.running.as_mut().unwrap().spinner_frame += 1;

        let status = {
            let task = self.running.as_mut().unwrap();
            task.child.try_wait()
        };

        match status {
            Ok(Some(exit_status)) => {
                let mut task = self.running.take().unwrap();
                let elapsed = task.started.elapsed();
                let duration = format!("{:.1}s", elapsed.as_secs_f64());

                let mut stdout_buf = String::new();
                let mut stderr_buf = String::new();
                if let Some(ref mut pipe) = task.child.stdout {
                    let _ = pipe.read_to_string(&mut stdout_buf);
                }
                if let Some(ref mut pipe) = task.child.stderr {
                    let _ = pipe.read_to_string(&mut stderr_buf);
                }

                if exit_status.success() {
                    let tail = tail_string(&stdout_buf, 1);
                    self.status_line = if tail.is_empty() {
                        format!("[OK] '{}' ({duration})", task.label)
                    } else {
                        format!("[OK] '{}' ({duration}): {tail}", task.label)
                    };
                } else {
                    let code = exit_status.code().unwrap_or(-1);
                    let tail = tail_string(&stderr_buf, 1);
                    self.status_line = if tail.is_empty() {
                        format!("[ERR] '{}' failed (exit {code}, {duration})", task.label)
                    } else {
                        format!("[ERR] '{}' (exit {code}, {duration}): {tail}", task.label)
                    };
                }

                self.refresh_all();
            }
            Ok(None) => {}
            Err(e) => {
                let label = self.running.take().unwrap().label;
                self.status_line = format!("[ERR] '{label}' poll error: {e}");
            }
        }
    }
}

// ── Action builders ───────────────────────────────────────

fn build_system_actions() -> Vec<SimpleAction> {
    let cli = "cargo run -p app-cli --manifest-path rust/Cargo.toml --";

    vec![
        SimpleAction {
            label: "Doctor".into(),
            description: "Run comprehensive health checks across all categories (packages, storage, services, stability, performance, security). Safe read-only operation.".into(),
            command: format!("{cli} doctor --full --root ."),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "Status".into(),
            description: "Display current system status including pending updates, failed services, orphan packages, and cache sizes.".into(),
            command: format!("{cli} run system-status --root ."),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "Update (preview)".into(),
            description: "Preview available updates without making changes. Shows Arch news and available AUR updates.".into(),
            command: format!("{cli} run system-update --dry-run --root ."),
            needs_confirm: false,
            is_preview: true,
        },
        SimpleAction {
            label: "Update (apply)".into(),
            description: "Apply system updates: sync keyring, run pacman -Syu, update AUR packages. Will modify system.".into(),
            command: format!("{cli} run system-update --apply --root ."),
            needs_confirm: true,
            is_preview: false,
        },
        SimpleAction {
            label: "Clean (preview)".into(),
            description: "Preview cleanup operations without deleting anything. Shows what would be removed.".into(),
            command: format!("{cli} run system-clean --dry-run --root ."),
            needs_confirm: false,
            is_preview: true,
        },
        SimpleAction {
            label: "Clean (apply)".into(),
            description: "Execute cleanup: package cache (keep 3 versions), orphans, journal (2 weeks max), old user cache.".into(),
            command: format!("{cli} run system-clean --apply --root ."),
            needs_confirm: true,
            is_preview: false,
        },
    ]
}

fn build_module_actions(_root: &Path) -> Vec<SimpleAction> {
    // Module actions are now handled via checkboxes
    // Return info action
    vec![
        SimpleAction {
            label: "Module Management".into(),
            description: "Use Space to toggle modules, then press Enter to apply changes. Active modules shown with ● indicator.".into(),
            command: "echo 'Module management via checkboxes'".into(),
            needs_confirm: false,
            is_preview: false,
        },
    ]
}

fn build_theme_actions(root: &Path) -> Vec<SimpleAction> {
    let mut actions = Vec::new();

    // Show current active theme
    if let Ok(Some(active)) = get_active_theme(root) {
        actions.push(SimpleAction {
            label: format!("Current: {}", active),
            description: format!("Currently active theme is '{}'", active),
            command: "echo 'Current theme info'".into(),
            needs_confirm: false,
            is_preview: false,
        });
    } else {
        actions.push(SimpleAction {
            label: "Current: <none>".into(),
            description: "No theme is currently active".into(),
            command: "echo 'No theme set'".into(),
            needs_confirm: false,
            is_preview: false,
        });
    }

    // Load and display available themes
    if let Ok(themes) = load_themes(root) {
        for theme in themes {
            let description = theme
                .description
                .as_deref()
                .unwrap_or("No description available");
            let cli = "cargo run -p app-cli --manifest-path rust/Cargo.toml --";

            actions.push(SimpleAction {
                label: format!("Switch to: {}", theme.name),
                description: format!(
                    "Switch to '{}' theme. {}. Shell: {}",
                    theme.name, description, theme.shell
                ),
                command: format!("{cli} theme switch {} --root .", theme.id),
                needs_confirm: true,
                is_preview: false,
            });
        }
    } else {
        actions.push(SimpleAction {
            label: "Error loading themes".into(),
            description: "Failed to load themes.toml. Check app/manifests/themes.toml".into(),
            command: "echo 'Error'".into(),
            needs_confirm: false,
            is_preview: false,
        });
    }

    actions
}

fn build_service_actions(root: &Path) -> Vec<SimpleAction> {
    let cli = "cargo run -p app-cli --manifest-path rust/Cargo.toml --";
    let mut actions = Vec::new();

    // List services (info)
    actions.push(SimpleAction {
        label: "List All Services".into(),
        description: "Show all configured services and their current status. Safe informational command.".into(),
        command: format!("{cli} service list --root ."),
        needs_confirm: false,
        is_preview: false,
    });

    // Apply defaults dry-run
    actions.push(SimpleAction {
        label: "Apply Defaults (preview)".into(),
        description: "Preview what service changes would be made WITHOUT applying them. Safe dry-run.".into(),
        command: format!("{cli} service apply-defaults --dry-run --root ."),
        needs_confirm: false,
        is_preview: true,
    });

    // Apply defaults
    actions.push(SimpleAction {
        label: "Apply Defaults".into(),
        description: "Apply default service configurations (enable/disable systemd units). This WILL modify systemd state.".into(),
        command: format!("{cli} service apply-defaults --root ."),
        needs_confirm: true,
        is_preview: false,
    });

    // Load services and add per-service actions
    if let Ok(services) = load_services(root) {
        for svc in services {
            if svc.service_type == "systemd-user" {
                let unit = svc.unit.as_deref().unwrap_or(&svc.id);

                actions.push(SimpleAction {
                    label: format!("{}: Status", svc.id),
                    description: format!("Check the current status of systemd unit '{}'. Safe informational command.", unit),
                    command: format!("systemctl --user status {} || true", unit),
                    needs_confirm: false,
                    is_preview: false,
                });

                actions.push(SimpleAction {
                    label: format!("{}: Enable", svc.id),
                    description: format!("Enable systemd unit '{}' to start automatically. This WILL modify systemd configuration.", unit),
                    command: format!("systemctl --user enable {}", unit),
                    needs_confirm: true,
                    is_preview: false,
                });

                actions.push(SimpleAction {
                    label: format!("{}: Disable", svc.id),
                    description: format!("Disable systemd unit '{}' from starting automatically. This WILL modify systemd configuration.", unit),
                    command: format!("systemctl --user disable {}", unit),
                    needs_confirm: true,
                    is_preview: false,
                });
            }
        }
    }

    actions
}

fn build_quickstart_actions() -> Vec<SimpleAction> {
    let cli = "cargo run -p app-cli --manifest-path rust/Cargo.toml --";

    vec![
        SimpleAction {
            label: "Step 1: Health Check".into(),
            description: "FIRST STEP: Run comprehensive health checks to verify your system is ready. Checks packages, storage, services, stability, performance, and security.".into(),
            command: format!("{cli} doctor --full --root ."),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "Step 2: Preview Updates".into(),
            description: "SECOND STEP: See what packages would be updated WITHOUT making changes. Reviews Arch news, checks AUR packages, and shows pre-flight status.".into(),
            command: format!("{cli} run system-update --dry-run --root ."),
            needs_confirm: false,
            is_preview: true,
        },
        SimpleAction {
            label: "Step 3: Preview Config".into(),
            description: "THIRD STEP: Preview quickstart configuration WITHOUT applying. Shows what modules, services, and configs would be set up.".into(),
            command: format!("{cli} quickstart --root ."),
            needs_confirm: false,
            is_preview: true,
        },
        SimpleAction {
            label: "Step 4: Apply Updates".into(),
            description: "Apply system updates (keyring first, then pacman -Syu, then AUR). This WILL modify your system packages.".into(),
            command: format!("{cli} run system-update --apply --root ."),
            needs_confirm: true,
            is_preview: false,
        },
        SimpleAction {
            label: "Step 5: Apply Config".into(),
            description: "Apply quickstart configuration including services and modules. This WILL modify your system files and systemd units.".into(),
            command: format!("{cli} quickstart --apply --root ."),
            needs_confirm: true,
            is_preview: false,
        },
        SimpleAction {
            label: "Step 6: Verify".into(),
            description: "FINAL STEP: Run full doctor check to confirm everything was set up correctly. Check for any new errors or warnings.".into(),
            command: format!("{cli} doctor --full --root ."),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "Optional: Preview Cleanup".into(),
            description: "Preview what would be cleaned up: orphan packages, old caches, journal entries. Safe to run anytime.".into(),
            command: format!("{cli} run system-clean --dry-run --root ."),
            needs_confirm: false,
            is_preview: true,
        },
        SimpleAction {
            label: "Optional: Apply Cleanup".into(),
            description: "Apply cleanup to reclaim disk space. Removes orphans, old package cache, journal entries, and user cache files.".into(),
            command: format!("{cli} run system-clean --apply --root ."),
            needs_confirm: true,
            is_preview: false,
        },
    ]
}

fn build_script_actions() -> Vec<SimpleAction> {
    vec![
        SimpleAction {
            label: "doctor-system.sh".into(),
            description: "Health verification. Flags: --apply (auto-fix), --json, --category=X".into(),
            command: "./scripts/doctor-system.sh --help".into(),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "update-system.sh".into(),
            description: "System updates. Flags: --apply (execute), --non-interactive".into(),
            command: "./scripts/update-system.sh --help".into(),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "clean-system.sh".into(),
            description: "Cleanup operations. Flags: --apply (execute), --aggressive".into(),
            command: "./scripts/clean-system.sh --help".into(),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "stow-modules.sh".into(),
            description: "Deploy dotfile modules via GNU Stow.".into(),
            command: "./scripts/stow-modules.sh --help".into(),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "switch-theme.sh".into(),
            description: "Switch system theme (gtk, terminal, etc).".into(),
            command: "./scripts/switch-theme.sh --help".into(),
            needs_confirm: false,
            is_preview: false,
        },
        SimpleAction {
            label: "install-packages.sh".into(),
            description: "Install packages from manifests.".into(),
            command: "./scripts/install-packages.sh --help".into(),
            needs_confirm: false,
            is_preview: false,
        },
    ]
}

// Removed - no longer needed with checkbox interface
// fn shorten_module_name(name: &str) -> String {
//     name.trim_end_matches(" Config Module")
//         .trim_end_matches(" Module")
//         .to_string()
// }

// ── Log reader ────────────────────────────────────────────

fn read_recent_ops(root: &Path, count: usize) -> Vec<OpLogEntry> {
    let path = root.join("app/state/logs/operations.jsonl");
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut entries: Vec<OpLogEntry> = raw
        .lines()
        .filter_map(|line| {
            let v = serde_json::from_str::<Value>(line).ok()?;
            Some(OpLogEntry {
                timestamp_ms: v.get("timestamp_unix_ms").and_then(Value::as_u64).unwrap_or(0),
                module: v.get("module").and_then(Value::as_str).unwrap_or("unknown").to_string(),
                operation: v.get("operation").and_then(Value::as_str).unwrap_or("unknown").to_string(),
                result: v.get("result").and_then(Value::as_str).unwrap_or("unknown").to_string(),
                exit_code: v.get("exit_code").and_then(Value::as_i64).unwrap_or(-1),
                duration_ms: v.get("duration_ms").and_then(Value::as_u64).unwrap_or(0),
            })
        })
        .collect();

    if entries.len() > count {
        entries = entries.split_off(entries.len() - count);
    }

    entries
}

// ── Helpers ───────────────────────────────────────────────

fn tail_string(input: &str, lines: usize) -> String {
    let mut split: Vec<&str> = input.lines().collect();
    if split.is_empty() {
        return String::new();
    }
    if split.len() > lines {
        split = split.split_off(split.len() - lines);
    }
    split.join(" | ")
}

fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn format_relative_time(now_ms: u64, then_ms: u64) -> String {
    if then_ms == 0 || now_ms < then_ms {
        return "just now".to_string();
    }
    let diff_s = (now_ms - then_ms) / 1000;
    if diff_s < 60 {
        format!("{diff_s}s ago")
    } else if diff_s < 3600 {
        format!("{}m ago", diff_s / 60)
    } else if diff_s < 86400 {
        format!("{}h ago", diff_s / 3600)
    } else {
        format!("{}d ago", diff_s / 86400)
    }
}

// ── Drawing ───────────────────────────────────────────────

fn draw_ui(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Tab bar
            Constraint::Length(5), // Status panels
            Constraint::Min(10),   // Body
            Constraint::Length(1), // Footer
        ])
        .split(frame.area());

    draw_title(frame, chunks[0]);
    draw_tab_bar(frame, app, chunks[1]);
    draw_status_panels(frame, app, chunks[2]);
    draw_body(frame, app, chunks[3]);
    draw_footer(frame, app, chunks[4]);

    if let Some(index) = app.confirm_pending {
        let label = app
            .current_actions()
            .get(index)
            .map(|a| a.label.as_str())
            .unwrap_or("selected action");
        draw_confirm_modal(frame, label);
    }
}

fn draw_title(frame: &mut ratatui::Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " JFF System TUI",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  |  Friendly Desktop Control Panel",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(title, area);
}

fn draw_tab_bar(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));

    for (idx, tab) in Tab::all().iter().enumerate() {
        let is_active = *tab == app.current_tab;
        let num = idx + 1;

        if is_active {
            spans.push(Span::styled(
                format!("[{} {}]", num, tab.label()),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} {} ", num, tab.label()),
                Style::default().fg(Color::White),
            ));
        }

        if idx < Tab::all().len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    let tab_bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(tab_bar, area);
}

fn draw_status_panels(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    let (health_symbol, health_label, health_color) = if app.validation_errors == 0 {
        ("[OK]", "Healthy", Color::Green)
    } else {
        ("[!!]", "Needs Attention", Color::Red)
    };

    let mut health_lines = vec![
        Line::from(vec![Span::styled(
            format!("{health_symbol} {health_label}"),
            Style::default()
                .fg(health_color)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!(
            "Errors: {}  Warnings: {}",
            app.validation_errors, app.validation_warnings
        )),
    ];
    if let Some(err) = &app.load_error {
        health_lines.push(Line::from(Span::styled(
            format!("! {err}"),
            Style::default().fg(Color::Yellow),
        )));
    }

    let left = Paragraph::new(health_lines)
        .block(Block::default().borders(Borders::ALL).title("Health"));
    frame.render_widget(left, status_chunks[0]);

    let middle = Paragraph::new(vec![
        Line::from(format!("Manifests: {}", app.manifests)),
        Line::from(format!("Modules: {}  Services: {}", app.modules, app.services)),
    ])
    .block(Block::default().borders(Borders::ALL).title("Inventory"));
    frame.render_widget(middle, status_chunks[1]);

    let right = Paragraph::new(vec![
        Line::from(format!("Host: {}", app.active_host)),
        Line::from(format!("Theme: {}", app.active_theme)),
        Line::from(Span::styled(
            format!("Root: {}", app.root.display()),
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title("Context"));
    frame.render_widget(right, status_chunks[2]);
}

fn draw_body(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left column: tab content (action list) + preview pane
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(6)])
        .split(body_chunks[0]);

    draw_tab_content(frame, app, left_chunks[0]);
    draw_preview_pane(frame, app, left_chunks[1]);

    // Right column: recent operations
    draw_log_pane(frame, app, body_chunks[1]);
}

fn draw_tab_content(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    // Special handling for Modules tab - show checkboxes
    if app.current_tab == Tab::Modules {
        draw_module_checkboxes(frame, app, area);
        return;
    }

    // Special handling for System tab - show maintenance status
    if app.current_tab == Tab::System {
        draw_system_with_status(frame, app, area);
        return;
    }

    let actions = app.current_actions();
    let state = app.current_tab_state();

    let items: Vec<ListItem> = actions
        .iter()
        .map(|action| {
            let (prefix, style) = if action.needs_confirm {
                ("* ", Style::default().fg(Color::Yellow))
            } else if action.is_preview {
                ("  ", Style::default().fg(Color::Green))
            } else {
                ("  ", Style::default().fg(Color::White))
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&action.label, style),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));

    let title = format!("{} ({})", app.current_tab.label(), actions.len());

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_system_with_status(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let actions = app.current_actions();
    let state = app.current_tab_state();

    // Get maintenance state
    let (clean_status, doctor_status, update_status) =
        get_maintenance_statuses(&app.root).unwrap_or((
            MaintenanceStatus::Never,
            MaintenanceStatus::Never,
            MaintenanceStatus::Never,
        ));

    let maint_state = get_maintenance_state(&app.root).ok();

    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let (prefix, style) = if action.needs_confirm {
                ("* ", Style::default().fg(Color::Yellow))
            } else if action.is_preview {
                ("  ", Style::default().fg(Color::Green))
            } else {
                ("  ", Style::default().fg(Color::White))
            };

            // Add status indicator based on operation type
            // New simplified action structure:
            // 0: Doctor, 1: Status, 2: Update preview, 3: Update apply, 4: Clean preview, 5: Clean apply
            let (status_tag, status_color, time_str) = match idx {
                0 => {
                    // Doctor - show last doctor run
                    let time = maint_state
                        .as_ref()
                        .and_then(|s| time_since_last(&s.last_doctor))
                        .map(format_time_ago)
                        .unwrap_or_else(|| "never".to_string());
                    let color = match doctor_status {
                        MaintenanceStatus::Recent => Color::Green,
                        MaintenanceStatus::NeedsAttention => Color::Yellow,
                        MaintenanceStatus::Overdue => Color::Red,
                        MaintenanceStatus::Never => Color::DarkGray,
                    };
                    (doctor_status.tag(), color, time)
                }
                3 => {
                    // Update (apply) - show last update run
                    let time = maint_state
                        .as_ref()
                        .and_then(|s| time_since_last(&s.last_update))
                        .map(format_time_ago)
                        .unwrap_or_else(|| "never".to_string());
                    let color = match update_status {
                        MaintenanceStatus::Recent => Color::Green,
                        MaintenanceStatus::NeedsAttention => Color::Yellow,
                        MaintenanceStatus::Overdue => Color::Red,
                        MaintenanceStatus::Never => Color::DarkGray,
                    };
                    (update_status.tag(), color, time)
                }
                5 => {
                    // Clean (apply) - show last clean run
                    let time = maint_state
                        .as_ref()
                        .and_then(|s| time_since_last(&s.last_clean))
                        .map(format_time_ago)
                        .unwrap_or_else(|| "never".to_string());
                    let color = match clean_status {
                        MaintenanceStatus::Recent => Color::Green,
                        MaintenanceStatus::NeedsAttention => Color::Yellow,
                        MaintenanceStatus::Overdue => Color::Red,
                        MaintenanceStatus::Never => Color::DarkGray,
                    };
                    (clean_status.tag(), color, time)
                }
                _ => ("", Color::DarkGray, String::new()),
            };

            let mut spans = vec![
                Span::styled(prefix, style),
                Span::styled(&action.label, style),
            ];

            if !status_tag.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(status_tag, Style::default().fg(status_color)));
                spans.push(Span::styled(format!(" {}", time_str), Style::default().fg(Color::DarkGray)));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));

    // Build title with summary status (text-based tags)
    let title = format!(
        "{} | D:{} U:{} C:{}",
        app.current_tab.label(),
        doctor_status.tag(),
        update_status.tag(),
        clean_status.tag()
    );

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_module_checkboxes(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let state = app.current_tab_state();

    let items: Vec<ListItem> = app
        .module_checkboxes
        .iter()
        .map(|(module, is_active)| {
            let checkbox = if *is_active { "●" } else { "○" };
            let checkbox_color = if *is_active {
                Color::Green
            } else {
                Color::DarkGray
            };

            ListItem::new(Line::from(vec![
                Span::styled(checkbox, Style::default().fg(checkbox_color)),
                Span::raw(" "),
                Span::styled(&module.name, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(
                    format!("({})", module.kind),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));

    let title = format!(
        "Modules ({}/{} active)",
        app.active_module_count, app.total_module_count
    );

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_preview_pane(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let description = if app.current_tab == Tab::Modules {
        // Show selected module info in Modules tab
        let state = app.current_tab_state();
        app.module_checkboxes
            .get(state.selected)
            .map(|(module, is_active)| {
                let status = if *is_active { "ACTIVE" } else { "INACTIVE" };
                format!(
                    "{}\n\nStatus: {}\nKind: {}\nID: {}\n\nOperations: {}",
                    module.name,
                    status,
                    module.kind,
                    module.id,
                    module
                        .operations
                        .iter()
                        .map(|op| op.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .unwrap_or_else(|| "No module selected".to_string())
    } else {
        app.current_action()
            .map(|a| a.description.clone())
            .unwrap_or_else(|| "Select an action to see its description.".to_string())
    };

    let preview = Paragraph::new(description)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Details"),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(preview, area);
}

fn draw_log_pane(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;

    if app.recent_ops.is_empty() {
        let empty = Paragraph::new("No operation entries yet.").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recent Operations"),
        );
        frame.render_widget(empty, area);
        return;
    }

    // Display newest-first
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let reversed: Vec<&OpLogEntry> = app.recent_ops.iter().rev().collect();

    let end = inner_height.min(reversed.len());
    let visible = &reversed[..end];

    let items: Vec<ListItem> = visible
        .iter()
        .map(|entry| {
            let (tag, tag_color) = if entry.result == "ok" {
                ("[OK] ", Color::Green)
            } else {
                ("[ERR]", Color::Red)
            };

            // Shorter operation name - just module, not module::operation
            let op_text = entry.module.clone();
            let duration_text = format_duration_ms(entry.duration_ms);
            let age_text = format_relative_time(now_ms, entry.timestamp_ms);

            ListItem::new(Line::from(vec![
                Span::styled(
                    tag,
                    Style::default()
                        .fg(tag_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:<28}", op_text),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!("{:>6}", duration_text),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw("  "),
                Span::styled(age_text, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let log_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recent Operations"),
    );
    frame.render_widget(log_list, area);
}

fn draw_footer(frame: &mut ratatui::Frame, app: &App, area: Rect) {
    // Running state - simple spinner display
    if let Some(task) = &app.running {
        let elapsed = task.started.elapsed().as_secs_f64();
        let spinners = ['◐', '◓', '◑', '◒'];
        let spinner = spinners[task.spinner_frame % spinners.len()];
        let content = Line::from(vec![
            Span::styled(
                format!(" {} ", spinner),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Running '{}' ({:.1}s)...", task.label, elapsed),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("[q]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" Quit", Style::default().fg(Color::Gray)),
        ]);
        let footer = Paragraph::new(content).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(footer, area);
        return;
    }

    // Context-aware help items based on current tab
    let sel = app.current_tab_state().selected;
    let help_items: Vec<(&str, &str)> = match app.current_tab {
        Tab::Modules => vec![
            ("Space", "Toggle"),
            ("Enter", "Apply"),
            ("↑↓", "Select"),
            ("←→", "Tab"),
            ("r", "Refresh"),
            ("q", "Quit"),
        ],
        Tab::System if sel == 4 || sel == 5 => vec![
            ("Enter", "Run"),
            ("Shift+Enter", "Aggressive"),
            ("↑↓", "Select"),
            ("←→", "Tab"),
            ("r", "Refresh"),
            ("q", "Quit"),
        ],
        _ => vec![
            ("Enter", "Run"),
            ("↑↓", "Select"),
            ("←→", "Tab"),
            ("1-6", "Jump"),
            ("r", "Refresh"),
            ("q", "Quit"),
        ],
    };

    // Build help spans with highlighted keys
    let mut spans = vec![Span::raw(" ")];

    // Add status message first if not default
    if !app.status_line.is_empty() && app.status_line != "Ready" {
        spans.push(Span::styled(
            &app.status_line,
            Style::default().fg(Color::Green),
        ));
        spans.push(Span::styled("  │  ", Style::default().fg(Color::DarkGray)));
    }

    for (i, (key, desc)) in help_items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            format!("[{}]", key),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(Color::Gray),
        ));
    }

    let footer = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(footer, area);
}

fn draw_confirm_modal(frame: &mut ratatui::Frame, label: &str) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let modal = Paragraph::new(vec![
        Line::from(Span::styled(
            "Confirm Action",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Run '{label}'?")),
        Line::from("This may apply system changes."),
        Line::from(""),
        Line::from("Press 'y' to confirm or 'n' to cancel."),
    ])
    .block(
        Block::default()
            .title("Confirmation")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(modal, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
