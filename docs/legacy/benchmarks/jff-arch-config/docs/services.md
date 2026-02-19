# Services Guide

This document describes how service management works for this project.

## Service registry
Primary registry file:
- `app/manifests/services.toml`

Current entries include:
- keyring session startup
- keyring health check
- systemd user services/timers for:
  - secrets check
  - update dry-run

## Unit sources
User units are stored in:
- `modules/services/systemd-user/units`

Units currently managed:
- `jff-secrets-check.service`
- `jff-secrets-check.timer`
- `jff-update-dry-run.service`
- `jff-update-dry-run.timer`

## Sync units
Use CLI sync first:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service sync-user-units --root .
```

Fallback script:

```bash
scripts/systemd-sync-user-units.sh
```

## Apply enabled defaults
Use one command to apply `enabled=true` services:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --root .
```

## Manual service control
```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service list --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service status secrets-check-timer --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service enable secrets-check-timer --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service start secrets-check-timer --root .
```

## Troubleshooting
If systemd user actions fail:
- Ensure command runs inside your desktop user session.
- Check user bus availability:
  - `busctl --user --list | grep -F org.freedesktop.secrets`
- Reload user units after changes:
  - `systemctl --user daemon-reload`
