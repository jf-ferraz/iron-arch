# Transitional Scripts

Shell scripts in this folder are transitional until the Rust CLI/TUI takes over execution.

Rules:
- Use `set -euo pipefail`
- Support `--dry-run`
- Write logs to `app/state/logs/`
- Use lock files in `app/state/locks/`
- Emit structured operation events to `app/state/logs/operations.jsonl`

## Quality gates
- `scripts/lint.sh`: shell + rust lint checks.
- `scripts/test.sh`: rust tests + CLI smoke tests.
- `scripts/ci-check.sh`: runs lint then test.

## Systemd helpers
- `scripts/systemd-sync-user-units.sh`: sync managed user units into `~/.config/systemd/user` and run `systemctl --user daemon-reload`.
