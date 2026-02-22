# jff-arch-config

Declarative Arch Linux desktop configuration and operations platform with a Rust CLI + TUI.

## WHAT THIS PROJECT SHOULD DO 
- Manage all Arch config files, such as: Niri, Hyperland, Waybar, etc
- Manage Arch system updates and packages 
- All solution must be designed for easy and extremely friendly usability, such as: 
  - Update all packages with a feature/option inside the TUI -> Update system (runs a robust script to ensure maximal security and stability)
  - Switch feature to rapidly Active/Deactive different config files across system (or change)
  - This is a personal APP, so the objective is to build an solution that will be my best friend for every day use. For example: Every day run on the terminal easy commands such as: 
  - `app-cli run system-update` -> update system with robust script
  - `app-cli run system-doctor` -> debug system and other analyzer features
  - `app-cli run system-clean` -> clean system cache and other powerful scripts to make system more realiable, fast and performatic
  - `app-cli run system-status` -> system current status - if needed update, current config, etc
  - `app-cli go` -> run TUI for fast and easy management
  - `app-cli run switcher shell--noctalia` // or `app-cli run switcher shell-dank`  -> Switcher will be a new feature that rapidly switcher from a X shell to Y shell. 
  - etc etc

## What This Project Does
- Manages Hyprland config reproducibly with Stow.
- Manages service/timer behavior from manifests.
- Provides safe operation flow: validate -> plan -> apply.
- Logs all major operations to JSONL for auditing.
- Provides beginner-friendly command flows (`doctor`, `quickstart`, `go`).

## Current Status
- Single-host setup: `desktop`
- Hyprland module operational
- Service management operational
- Update workflow operational (safe by default)
- TUI dashboard operational

## Project Layout
- `app/manifests/`: declarative policy and registries
- `modules/`: module contracts and assets
- `hosts/`: host overlays (current: `desktop`)
- `scripts/`: shell tooling and adapters
- `rust/`: `app-cli`, `app-tui`, core domain crates
- `docs/`: full operational documentation

## Start Here
1. Read `docs/bootstrap.md`.
2. Run health check:

```bash
cd /home/laraj/Documents/jff-arch-config
cargo run -p app-cli --manifest-path rust/Cargo.toml -- doctor --root .
```

3. Run guided dry-run flow:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- quickstart --root .
```

4. Apply when ready:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- quickstart --apply --root .
```

## Useful Commands
```bash
# Fast guided mode
cargo run -p app-cli --manifest-path rust/Cargo.toml -- go --root .

# Plan and apply Hyprland
cargo run -p app-cli --manifest-path rust/Cargo.toml -- plan --module hyprland --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --root .

# Service defaults
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --root .

# Launch TUI
cargo run -p app-tui --manifest-path rust/Cargo.toml -- --root .
```

## Documentation Map
- `docs/README.md`
- `docs/bootstrap.md`
- `docs/cli.md`
- `docs/tui.md`
- `docs/services.md`
- `docs/architecture.md`
- `docs/recovery.md`
