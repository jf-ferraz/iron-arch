#!/usr/bin/env bash
#
# test-apply.sh — validate `iron apply` for the imported dotfiles profile on Arch.
#
# Deploys the full "main" profile into a throwaway sandbox $HOME, so your real
# ~/.config is NEVER touched. Idempotent and safe to re-run: each run rebuilds
# iron, re-initializes the host fresh (so you also see the fixed monitor catalog),
# applies the profile into the sandbox, and checks that the 4 vendored base16
# theme paths resolved.
#
# Usage:   bash scripts/test-apply.sh
# Override paths via env, e.g.:  CFG=~/iron SANDBOX=/tmp/it REPO=~/src/iron-arch bash scripts/test-apply.sh
#
set -Eeuo pipefail

REPO="${REPO:-$HOME/dev/projects/iron-arch}"   # iron tool source (branch: dev)
CFG="${CFG:-$HOME/iron}"                        # iron-config dotfiles repo
SANDBOX="${SANDBOX:-/tmp/iron-apply-test}"      # throwaway deploy target ($HOME)
IRON="$REPO/target/release/iron"
HOST_ID="$(hostname)"

h()  { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }
ok() { printf '   \033[32m✓\033[0m %s\n' "$*"; }
no() { printf '   \033[31m✗\033[0m %s\n' "$*" >&2; }

# 1) Build iron natively (the binary is host/glibc-specific — build where you run).
h "Building iron (release) from $REPO"
git -C "$REPO" log -1 --oneline || true
( cd "$REPO" && cargo build --release )
ok "built $IRON"
"$IRON" --version

# 2) Ensure the config repo is present and current.
if [ -d "$CFG/.git" ]; then
  h "Updating config repo $CFG"
  git -C "$CFG" pull --ff-only || no "pull skipped (local changes?) — continuing"
else
  h "Cloning iron-config -> $CFG"
  git clone https://github.com/jf-ferraz/iron-config "$CFG"
fi

# 3) Fresh host init. This exercises the FIXED monitor catalog and produces a host
#    file WITHOUT a 'profile' key, which we then prepend at top level — dodging the
#    TOML "scalar after table" rule that bites if a tool re-serializes the host.
h "Initializing host '$HOST_ID' (fresh, --force)"
rm -f "$CFG"/hosts/*.toml 2>/dev/null || true
# --force: `iron init` is a no-op (exit 0, no host file) if already initialized,
# since init-state lives outside hosts/*.toml. --force regenerates regardless.
"$IRON" --root "$CFG" init --force </dev/null
HOSTFILE="$CFG/hosts/$HOST_ID.toml"
[ -f "$HOSTFILE" ] || HOSTFILE="$(find "$CFG/hosts" -name '*.toml' 2>/dev/null | head -n1)"
if [ -z "${HOSTFILE:-}" ] || [ ! -f "$HOSTFILE" ]; then
  no "host file was not created at $CFG/hosts/ — init likely failed; aborting"
  exit 1
fi
ok "host file: $HOSTFILE"

# 3a) Sanity-check the monitor catalog (the bug we fixed produced ~78 phantom rows).
MON_COUNT="$(grep -c '^\[\[hardware.monitors\]\]' "$HOSTFILE" || true)"
h "Cataloged monitors: ${MON_COUNT:-0}"
grep 'output =' "$HOSTFILE" || true
if [ "${MON_COUNT:-0}" -gt 8 ]; then
  no "suspiciously many monitors — is the built iron on the fixed 'dev'? (HEAD above)"
else
  ok "monitor count looks sane"
fi

# 4) Declare the profile at the very top (before any [table]).
if ! head -n1 "$HOSTFILE" | grep -q '^profile'; then
  sed -i '1i profile = "main"' "$HOSTFILE"
fi
ok "profile line: $(grep -m1 '^profile' "$HOSTFILE")"

# 5) Dry-run — preview only, changes nothing.
h "iron apply --dry-run (preview only)"
"$IRON" --root "$CFG" apply --dry-run -v || no "dry-run returned non-zero (inspect above)"

# 6) Apply the full profile into a sandbox HOME. Real ~/.config is untouched.
h "Applying full 'main' profile into sandbox HOME=$SANDBOX"
rm -rf "$SANDBOX"; mkdir -p "$SANDBOX"
HOME="$SANDBOX" "$IRON" --root "$CFG" apply -y -v

# 7) Verify the deploy + the 4 vendored base16 theme paths resolved in the sandbox.
h "Verifying deployed configs + vendored theme paths"
chk() { if [ -e "$2" ]; then ok "$1 -> $2"; else no "$1 MISSING -> $2"; fi; }
chk "kitty config"                     "$SANDBOX/.config/kitty/kitty.conf"
chk "kitty base16 (relative include)"  "$SANDBOX/.config/kitty/colors-base16.conf"
chk "yazi syntect theme"               "$SANDBOX/.config/yazi/base16-fer-glass.tmTheme"
chk "fish base16 (sourced)"            "$SANDBOX/.config/fish/colors-base16.fish"
chk "fastfetch logo (~ expansion)"     "$SANDBOX/.config/fastfetch/logo.txt"

h "Deployed tree (sandbox, depth 2)"
find "$SANDBOX/.config" -maxdepth 2 2>/dev/null | sort || true

h "Done"
echo "   Sandbox: $SANDBOX   (remove with: rm -rf '$SANDBOX')"
echo "   Your real \$HOME/.config was NOT modified."
