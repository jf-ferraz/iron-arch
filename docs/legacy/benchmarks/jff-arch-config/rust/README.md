# Rust Workspace

Planned layering:
- `core-domain`: manifests, planning, validation.
- `infra-fs`: filesystem adapter.
- `infra-systemd`: service/timer adapter.
- `app-cli`: command entrypoint.
- `app-tui`: terminal UI entrypoint.

## Current CLI commands
- `cargo run -p app-cli -- status --root ..`
- `cargo run -p app-cli -- validate --root ..`
- `cargo run -p app-cli -- doctor --root ..`
- `cargo run -p app-cli -- quickstart --root ..`
- `cargo run -p app-cli -- quickstart --apply --root ..`
- `cargo run -p app-cli -- go --root ..`
- `cargo run -p app-cli -- go --apply --root ..`
- `cargo run -p app-cli -- host list --root ..`
- `cargo run -p app-cli -- host show --root ..`
- `cargo run -p app-cli -- host set desktop --root ..`
- `cargo run -p app-cli -- service list --root ..`
- `cargo run -p app-cli -- service sync-user-units --root ..`
- `cargo run -p app-cli -- service apply-defaults --dry-run --root ..`
- `cargo run -p app-cli -- service apply-defaults --root ..`
- `cargo run -p app-cli -- service status secrets-check-timer --root ..`
- `cargo run -p app-cli -- service enable secrets-check-timer --root ..`
- `cargo run -p app-cli -- service start secrets-check-timer --root ..`
- `cargo run -p app-cli -- plan --root ..`
- `cargo run -p app-cli -- plan --module hyprland --root ..`
- `cargo run -p app-cli -- apply hyprland --dry-run --root ..`
- `cargo run -p app-cli -- apply updates --operation dry-run --root ..`

## TUI
- Start dashboard:
  - `cargo run -p app-tui -- --root ..`
- Controls:
  - `q` quit
  - `r` refresh data
  - `Tab` switch focus (Actions/Logs)
  - `Up/Down` navigate actions or logs
  - `Enter` run selected action
  - `y/n` confirm or cancel apply actions
