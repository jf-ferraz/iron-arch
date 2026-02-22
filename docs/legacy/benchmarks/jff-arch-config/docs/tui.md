# TUI Guide

The TUI is a friendly control panel over the same CLI/core domain.

## Start
From project root:

```bash
cd /home/laraj/Documents/jff-arch-config
cargo run -p app-tui --manifest-path rust/Cargo.toml -- --root .
```

Alternative: run from the Rust workspace directory:

```bash
cd rust
cargo run -p app-tui -- --root ..
```

## Layout
- **Overview**: title and context.
- **Health/Inventory/Context**: validation and system summary.
- **Actions pane**: run common workflows quickly.
- **Recent Operations pane**: live log feed from `operations.jsonl`.
- **Controls footer**: key hints + latest action result.

## Controls
- `q`: quit
- `r`: refresh full dashboard data
- `Tab`: switch focus between Actions and Logs
- `Up/Down`: navigate selected pane
- `Enter`: run selected action
- `y/n`: confirm or cancel apply actions

## Action safety
Actions that perform real changes require confirmation.
Dry-run actions are available for safe preview.

## Recommended TUI workflow
1. Run `Doctor Check`.
2. Run `Plan Hyprland`.
3. Run `Apply Hyprland (Dry-Run)`.
4. Run `Apply Service Defaults (Dry-Run)`.
5. Run apply actions only after dry-runs are clean.
