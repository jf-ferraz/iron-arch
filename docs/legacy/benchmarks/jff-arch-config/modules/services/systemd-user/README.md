# Systemd User Services Module

This module tracks user-session services and timers.

## Managed units
- `jff-secrets-check.service`
- `jff-secrets-check.timer`
- `jff-update-dry-run.service`
- `jff-update-dry-run.timer`

Unit source directory:
- `modules/services/systemd-user/units`

## Sync units
- `cargo run -p app-cli --manifest-path rust/Cargo.toml -- service sync-user-units --root .`
- (fallback) `scripts/systemd-sync-user-units.sh`

## Manage services (CLI)
- `cargo run -p app-cli --manifest-path rust/Cargo.toml -- service list --root .`
- `cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --dry-run --root .`
- `cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --root .`
- `cargo run -p app-cli --manifest-path rust/Cargo.toml -- service status secrets-check-timer --root .`
- `cargo run -p app-cli --manifest-path rust/Cargo.toml -- service enable secrets-check-timer --root .`
- `cargo run -p app-cli --manifest-path rust/Cargo.toml -- service start secrets-check-timer --root .`

## Secret Service checks
- `scripts/check-secrets-service.sh`
