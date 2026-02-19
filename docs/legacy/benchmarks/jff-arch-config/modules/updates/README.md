# Updates Module

Stability-first Arch maintenance workflow implemented by `scripts/update-system.sh`.

## Key behaviors
- Single-run lock (`app/state/locks/update.lock`)
- Structured daily logs (`app/state/logs/update-YYYY-MM-DD.log`)
- Preflight checks (Arch, network, pacman lock)
- Arch News check + state tracking
- Optional mirror refresh (`reflector`)
- Full upgrade gate (`pacman -Syu` only)
- `.pacnew`/`.pacsave` detection
- Service health checks
- Orphan report + optional cache prune
- Optional boot-time benchmark logging

## Commands
- Dry run (default-safe):
  - `scripts/update-system.sh --dry-run`
- Apply updates:
  - `scripts/update-system.sh --apply`
- Apply non-interactive:
  - `scripts/update-system.sh --apply --non-interactive`

## Policy
Tune behavior in `app/manifests/update-policy.toml`.
