# Hyprland Module

This module is deployed with GNU Stow.

## Structure
- `stow/.config/hypr/`: canonical Hyprland config files.
- `hosts/<host>/hyprland/overrides.conf`: host-specific overrides.
- `scripts/hypr-sync-host-overlay.sh`: renders selected host override into `host-overrides.conf`.
- `scripts/deploy-hypr.sh`: conflict-aware deploy wrapper.

## Deploy Flow
- Dry run (recommended first):
  - `scripts/deploy-hypr.sh --dry-run`
- Apply:
  - `scripts/deploy-hypr.sh`
- Apply and adopt existing files:
  - `scripts/deploy-hypr.sh --adopt`
- Status:
  - `scripts/deploy-hypr.sh --status`
- Rollback:
  - `scripts/deploy-hypr.sh --rollback`

Host resolution order:
- `--host <id>`
- `HOST_ID` environment variable
- `app/state/run/active-host` (set by `app-cli host set <id>` or `scripts/host.sh set <id>`)
- `app/manifests/hosts.toml` `default_host`

Current setup is single-host with `desktop` as default and active host.

## Config split
- `00-env.conf`
- `10-input.conf`
- `20-monitors.conf`
- `30-workspaces.conf`
- `40-keybinds.conf`
- `50-window-rules.conf`
- `90-autostart.conf`
- `host-overrides.conf` (generated)
