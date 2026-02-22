# Bootstrap Guide

This project manages desktop configuration and system operations for an Arch Linux single-host setup (`desktop`).

## 1) Prerequisites
Install required tools:

```bash
sudo pacman -S --needed rust stow
```

Recommended quality tooling:

```bash
sudo pacman -S --needed shellcheck shfmt
```

Optional but useful packages:

```bash
sudo pacman -S --needed pacman-contrib reflector
```

## 2) Verify repository layout
From project root:

```bash
cd /home/laraj/Documents/jff-arch-config
ls app/manifests modules scripts rust
```

## 3) Validate repository health
Run doctor first:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- doctor --root .
```

Expected result: `Doctor Summary: 0 error(s), 0 warning(s)`.

Alternative: run from the Rust workspace directory:

```bash
cd rust
cargo run -p app-cli -- doctor --root ..
```

## 4) Run guided safe flow
Use dry-run onboarding:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- quickstart --root .
```

If successful, apply changes:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- quickstart --apply --root .
```

## 5) Hyprland deployment only (manual path)
If you want only Hyprland actions:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- plan --module hyprland --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --root .
```

## 6) Service defaults
Apply enabled services from `app/manifests/services.toml`:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --root .
```

## 7) Quality gates
Run lint/test suites:

```bash
scripts/lint.sh
scripts/test.sh
scripts/ci-check.sh
```
