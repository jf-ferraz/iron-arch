# CLI Guide

All commands below are run from project root:

```bash
cd /home/laraj/Documents/jff-arch-config
```

Use this prefix consistently:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- <command> --root .
```

Alternative: run from the Rust workspace directory:

```bash
cd rust
cargo run -p app-cli -- <command> --root ..
```

## User Journey Diagram
```text
doctor
  |
  v
quickstart (dry-run)
  |
  v
plan --module hyprland
  |
  v
apply hyprland --dry-run
  |
  v
apply hyprland
  |
  v
service apply-defaults
```

## Beginner commands

### doctor
Health check with clear guidance.

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- doctor --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- doctor --strict --root .
```

### quickstart / go
Guided workflow (dry-run by default).

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- quickstart --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- quickstart --apply --root .

cargo run -p app-cli --manifest-path rust/Cargo.toml -- go --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- go --apply --root .
```

## Core operational commands

### validate / status
```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- validate --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- status --root .
```

### plan
```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- plan --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- plan --module hyprland --root .
```

### apply
```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --root .

cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply updates --operation dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply updates --operation run --root .
```

## Host commands
```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- host list --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- host show --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- host set desktop --root .
```

## Service commands
```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service list --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service sync-user-units --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --root .
```

Targeted service control:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service status secrets-check-timer --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service enable secrets-check-timer --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service start secrets-check-timer --root .
```

## Exit code meanings
- `0`: success
- `1`: runtime failure
- `2`: usage/argument error
- `3`: validation failure
- `4`: module/operation not found
- `5`: operation command failed
