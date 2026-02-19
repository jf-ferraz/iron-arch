# Troubleshooting

## `cargo`: could not find `Cargo.toml`

You are in repo root, but the Rust workspace is in `rust/`.

Use either:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- doctor --root .
```

Or:

```bash
cd rust
cargo run -p app-cli -- doctor --root ..
```

## Stow conflict on `~/.config/hypr/hyprland.conf`

If a real file exists, stow will not overwrite it.

Recommended safe fix:

```bash
mv ~/.config/hypr/hyprland.conf ~/.config/hypr/hyprland.conf.pre-stow
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --dry-run --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- apply hyprland --root .
```

## Secret storage errors (no keyring / secrets service)

Check Secret Service:

```bash
scripts/check-secrets-service.sh
```

If it fails under Hyprland:
- Ensure `gnome-keyring-daemon --start --components=secrets` is in Hyprland autostart.
- Re-login into your Hyprland session.

## `systemctl --user` errors (user bus)

If `systemctl --user` fails, you are likely running outside a user session (TTY without user bus, sudo session, or remote environment).

Confirm the user bus is available:

```bash
busctl --user --list | grep -F org.freedesktop.secrets
```

Then sync units and retry:

```bash
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service sync-user-units --root .
cargo run -p app-cli --manifest-path rust/Cargo.toml -- service apply-defaults --root .
```

## Update workflow says network unavailable

Dry-run does connectivity checks. Fix networking first, then retry:

```bash
scripts/update-system.sh --dry-run
```

If you are intentionally offline, do not run apply; keep it dry-run only.

## `.pacnew` / `.pacsave` found after updates

Review/merge safely:

```bash
sudo pacdiff
```

