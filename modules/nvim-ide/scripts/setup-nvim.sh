#!/usr/bin/env bash
#
# Iron Post-Install: Neovim IDE Setup
# Installs plugins and configures LSP servers

set -euo pipefail

echo "Setting up Neovim IDE module..."

# Install language servers via npm
if command -v npm &>/dev/null; then
    echo "Installing language servers..."
    npm install -g typescript typescript-language-server
    npm install -g vscode-langservers-extracted
    npm install -g @tailwindcss/language-server
    npm install -g pyright
fi

# Install Rust analyzer if rustup is available
if command -v rustup &>/dev/null; then
    echo "Installing rust-analyzer..."
    rustup component add rust-analyzer
fi

# Run Neovim headless to install plugins
echo "Installing Neovim plugins (this may take a moment)..."
nvim --headless "+Lazy! sync" +qa 2>/dev/null || true

echo "Neovim IDE setup complete!"
echo "Open nvim to start using your IDE configuration."
