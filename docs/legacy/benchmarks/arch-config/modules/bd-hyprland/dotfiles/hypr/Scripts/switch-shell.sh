#!/bin/bash
# Shell Switching Script for Hyprland
# Switches between Noctalia, DMS, and Ax-Shell with fzf TUI

# Configuration
STATE_FILE="$HOME/.config/hypr/current_shell"
HYPR_DIR="$HOME/.config/hypr"
SYMLINK="$HYPR_DIR/keybinds-active.conf"
HYPRLAND_CONF="$HYPR_DIR/hyprland.conf"

# Shell configuration (easily extensible for future shells)
declare -A SHELL_START_CMD=(
    ["noctalia"]="qs -c noctalia-shell"
    ["dms"]="dms run"
    ["ax-shell"]="uwsm-app \$(python /home/don/.config/Ax-Shell/main.py)"
)

declare -A SHELL_KILL_PATTERN=(
    ["noctalia"]="qs -c noctalia-shell"
    ["dms"]="dms run"
    ["ax-shell"]="ax-shell"
)

declare -A SHELL_KEYBIND_FILE=(
    ["noctalia"]="$HYPR_DIR/keybinds-noctalia.conf"
    ["dms"]="$HYPR_DIR/keybinds-dms.conf"
    ["ax-shell"]="$HYPR_DIR/keybinds-ax-shell.conf"
)

declare -A SHELL_DISPLAY_NAME=(
    ["noctalia"]="Noctalia"
    ["dms"]="DMS"
    ["ax-shell"]="Ax-Shell"
)

# Function to enable/disable ax-shell.conf sourcing
toggle_axshell_source() {
    local action="$1"  # "enable" or "disable"

    if [ "$action" = "enable" ]; then
        # Uncomment the source line if commented
        sed -i 's|^# *source = ~/.config/Ax-Shell/config/hypr/ax-shell.conf|source = ~/.config/Ax-Shell/config/hypr/ax-shell.conf|' "$HYPRLAND_CONF"
    else
        # Comment out the source line
        sed -i 's|^source = ~/.config/Ax-Shell/config/hypr/ax-shell.conf|# source = ~/.config/Ax-Shell/config/hypr/ax-shell.conf|' "$HYPRLAND_CONF"
    fi
}

# Create state file if it doesn't exist (default to dms)
if [ ! -f "$STATE_FILE" ]; then
    echo "dms" > "$STATE_FILE"
fi

# Read current shell
CURRENT_SHELL=$(cat "$STATE_FILE")

# Build fzf options list with current shell highlighted
SHELL_OPTIONS=(
    "noctalia"
    "dms"
    "ax-shell"
)

# Use fzf to select target shell
TARGET_SHELL=$(printf "%s\n" "${SHELL_OPTIONS[@]}" | \
    fzf --prompt="Select Shell: " \
        --height=40% \
        --border=rounded \
        --reverse \
        --header="Current: ${SHELL_DISPLAY_NAME[$CURRENT_SHELL]}" \
        --color="border:#f5c2e7,header:#cba6f7,prompt:#f5c2e7,pointer:#f5c2e7")

# Exit if no selection made (user pressed ESC)
if [ -z "$TARGET_SHELL" ]; then
    echo "No shell selected. Exiting."
    exit 0
fi

# Exit if same shell selected
if [ "$TARGET_SHELL" = "$CURRENT_SHELL" ]; then
    notify-send "Shell Switcher" "Already using ${SHELL_DISPLAY_NAME[$TARGET_SHELL]}" -t 2000 -u normal
    exit 0
fi

# Get display names
CURRENT_NAME="${SHELL_DISPLAY_NAME[$CURRENT_SHELL]}"
TARGET_NAME="${SHELL_DISPLAY_NAME[$TARGET_SHELL]}"

# Show notification before switching
notify-send "Shell Switcher" "Switching from $CURRENT_NAME to $TARGET_NAME..." -t 2000 -u normal
sleep 2

# Kill current shell process
echo "Killing $CURRENT_NAME shell..."
pkill -f "${SHELL_KILL_PATTERN[$CURRENT_SHELL]}" 2>/dev/null
sleep 0.5

# Handle Ax-Shell source line toggling
if [ "$TARGET_SHELL" = "ax-shell" ]; then
    echo "Enabling Ax-Shell configuration..."
    toggle_axshell_source "enable"
else
    echo "Disabling Ax-Shell configuration..."
    toggle_axshell_source "disable"
fi

# Update state file
echo "$TARGET_SHELL" > "$STATE_FILE"

# Update symlink to point to new shell's keybind file
ln -sf "${SHELL_KEYBIND_FILE[$TARGET_SHELL]}" "$SYMLINK"

# Reload Hyprland configuration to apply new keybinds
hyprctl reload

# Start new shell (detached from terminal)
echo "Starting $TARGET_NAME shell..."
setsid -f bash -c "${SHELL_START_CMD[$TARGET_SHELL]}" >/dev/null 2>&1

# Wait for new shell to initialize
sleep 2

# Show completion notification
notify-send "Shell Switcher" "Switched to $TARGET_NAME" -t 2000 -u normal

echo "Successfully switched from $CURRENT_NAME to $TARGET_NAME"

# Wait 1 more second then exit (total 3 seconds)
sleep 1
exit 0
