# Recovery Playbook

This playbook defines rollback and incident response for desktop config and maintenance operations.

## 1) If Hyprland config breaks
Boot into TTY or another desktop session and run:

```bash
cd /home/laraj/Documents/jff-arch-config
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --operation rollback --root .
```

Then restore previous config if needed:

```bash
mv ~/.config/hypr/hyprland.conf.pre-stow-* ~/.config/hypr/hyprland.conf
```

## 2) If systemd user services misbehave
Disable problematic units:

```bash
systemctl --user disable --now jff-secrets-check.timer
systemctl --user disable --now jff-update-dry-run.timer
```

Re-sync and reload units:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service sync-user-units --root .
```

## 3) If update workflow fails
Run dry-run first to inspect:

```bash
scripts/update-system.sh --dry-run
```

Review latest logs:

```bash
ls -1 app/state/logs/update-*.log | tail -n 1
```

## 4) Package recovery patterns (Arch)
- Check pacman lock: `/var/lib/pacman/db.lck`
- Review `.pacnew/.pacsave` results from update logs
- Reconcile configs using `sudo pacdiff` if needed
- Reboot if kernel/glibc/systemd updates were applied

## 5) Before enabling unattended update operations
Have these in place:
- Snapshot strategy (Btrfs snapshots, Timeshift, or equivalent)
- Clear rollback command list in this document
- Periodic `doctor` and `ci-check` runs

## 6) Quick safety commands
```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- doctor --strict --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- quickstart --root .
```

If either command reports errors, resolve before applying changes.
