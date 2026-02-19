#!/bin/bash
# Toggle Niri gaming mode - adds spacing between monitors to trap cursor on main display
# Bind to Ctrl+Shift+G for quick gaming mode toggle

STATE_FILE="${XDG_RUNTIME_DIR:-/tmp}/niri-gaming-mode-state"

# Normal positions (from your config)
NORMAL_DP1_X=2740
NORMAL_DP1_Y=1455
NORMAL_DP2_X=820
NORMAL_DP2_Y=1714
NORMAL_DP3_X=6180
NORMAL_DP3_Y=1714

# Gaming positions (adds large gaps to prevent cursor movement)
# DP-1 stays in same position, but DP-2 and DP-3 move far away
GAMING_DP1_X=2740
GAMING_DP1_Y=1455
GAMING_DP2_X=-10000   # Move left monitor far to the left
GAMING_DP2_Y=1714
GAMING_DP3_X=20000    # Move right monitor far to the right
GAMING_DP3_Y=1714

# Check if niri is available
if ! command -v niri &> /dev/null; then
    notify-send "Gaming Mode" "niri not found. Please ensure niri is running." -u critical
    exit 1
fi

if [ -f "$STATE_FILE" ]; then
    # Gaming mode is ON, switch to NORMAL
    echo "Switching to NORMAL mode - monitors adjacent"
    niri msg output DP-1 position set -- $NORMAL_DP1_X $NORMAL_DP1_Y 2>/dev/null
    niri msg output DP-2 position set -- $NORMAL_DP2_X $NORMAL_DP2_Y 2>/dev/null
    niri msg output DP-3 position set -- $NORMAL_DP3_X $NORMAL_DP3_Y 2>/dev/null
    rm "$STATE_FILE"
    notify-send "Gaming Mode OFF" "Monitors restored to normal positions" -i input-gaming 2>/dev/null || notify-send "Gaming Mode OFF" "Monitors restored to normal positions"
else
    # Gaming mode is OFF, switch to GAMING
    echo "Switching to GAMING mode - cursor trapped on main monitor"
    niri msg output DP-1 position set -- $GAMING_DP1_X $GAMING_DP1_Y 2>/dev/null
    niri msg output DP-2 position set -- $GAMING_DP2_X $GAMING_DP2_Y 2>/dev/null
    niri msg output DP-3 position set -- $GAMING_DP3_X $GAMING_DP3_Y 2>/dev/null
    touch "$STATE_FILE"
    notify-send "Gaming Mode ON" "Cursor confined to main monitor (DP-1)" -i input-gaming 2>/dev/null || notify-send "Gaming Mode ON" "Cursor confined to main monitor (DP-1)"
fi















