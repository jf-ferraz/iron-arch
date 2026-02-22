#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Configuring Fish shell..."

FISH_DIR="$HOME/.config/fish"
mkdir -p "$FISH_DIR/conf.d" "$FISH_DIR/functions"

# Add dev/bin to PATH
ENV_FILE="$FISH_DIR/conf.d/env.fish"
if [[ ! -f "$ENV_FILE" ]] || ! grep -q "dev/bin" "$ENV_FILE" 2>/dev/null; then
    cat <<'FISH' >> "$ENV_FILE"
# Iron: dev tools path
fish_add_path ~/dev/bin
FISH
fi

# Add useful aliases
ALIAS_FILE="$FISH_DIR/conf.d/aliases.fish"
if [[ ! -f "$ALIAS_FILE" ]]; then
    cat <<'FISH' > "$ALIAS_FILE"
# Iron: dev aliases
alias cdev "cd ~/dev"
alias crepos "cd ~/dev/repos"
alias csync "cd ~/sync"
alias cdocs "cd ~/docs"
FISH
fi

# Create 'take' function (mkdir + cd)
cat <<'FISH' > "$FISH_DIR/functions/take.fish"
function take --description "Create directory and cd into it"
    mkdir -p $argv[1]; and cd $argv[1]
end
FISH

log_success "Fish shell configured"
