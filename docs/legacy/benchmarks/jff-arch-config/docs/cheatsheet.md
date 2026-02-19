# Cheatsheet

All commands assume you are in:

`/home/laraj/Documents/jff-arch-config`

## Safe Start

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- doctor --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- go --root .
```

## Apply (When Ready)

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- go --apply --root .
```

## Hyprland Only

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- plan --module hyprland --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --root .
```

## Services

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service list --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --root .
```

## TUI

```bash
cargo run -p app-tui --manifest-path rust/Cargo.toml -- --root .
```

## Quality Gates

```bash
scripts/ci-check.sh
```

