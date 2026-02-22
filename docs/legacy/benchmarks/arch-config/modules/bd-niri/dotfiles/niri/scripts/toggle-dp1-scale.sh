#!/bin/bash
# Toggle DP-1 monitor scale between 1 and 1.5

CONFIG_FILE="$HOME/.config/niri/outputs.kdl"

# Get current scale value
current_scale=$(grep -A 3 'output "DP-1"' "$CONFIG_FILE" | grep 'scale' | awk '{print $2}')

# Toggle between 1 and 1.25
if [ "$current_scale" = "1" ]; then
    new_scale="1.25"
else
    new_scale="1"
fi

# Update the config file
sed -i '/output "DP-1"/,/^}/ s/scale [0-9.]\+/scale '"$new_scale"'/' "$CONFIG_FILE"

# Reload niri configuration
niri msg reload-config

# Optional: Show notification
if command -v notify-send &> /dev/null; then
    notify-send "DP-1 Scale" "Changed to ${new_scale}x" -t 2000
fi
