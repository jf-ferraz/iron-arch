#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../scripts/lib/iron-hooks-common.sh"

log_info "Setting up SSH configuration..."

SSH_DIR="$HOME/.ssh"
mkdir -p "$SSH_DIR"
chmod 700 "$SSH_DIR"

# Generate SSH key if none exists
if [[ ! -f "$SSH_DIR/id_ed25519" ]]; then
    log_info "Generating ed25519 SSH key..."
    ssh-keygen -t ed25519 -f "$SSH_DIR/id_ed25519" -N "" -C "$USER@$(hostname)"
    log_success "SSH key generated: $SSH_DIR/id_ed25519.pub"
fi

# Create basic SSH config if missing
if [[ ! -f "$SSH_DIR/config" ]]; then
    cat <<'SSH' > "$SSH_DIR/config"
# Iron SSH client configuration
Host *
    AddKeysToAgent yes
    IdentityFile ~/.ssh/id_ed25519
    ServerAliveInterval 60
    ServerAliveCountMax 3
SSH
    chmod 600 "$SSH_DIR/config"
    log_info "Created SSH client config"
fi

log_success "SSH configuration complete"
log_info "Add your public key to remote hosts with: ssh-copy-id user@host"
