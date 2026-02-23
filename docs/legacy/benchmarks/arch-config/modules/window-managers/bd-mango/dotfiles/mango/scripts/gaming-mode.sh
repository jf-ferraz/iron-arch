#!/bin/bash
# Toggle Mango gaming mode - adds spacing between monitors to trap cursor on main display
# Bind to Ctrl+Shift+G for quick gaming mode toggle

STATE_FILE="${XDG_RUNTIME_DIR:-/tmp}/mango-gaming-mode-state"

# Normal positions (from niri config)
NORMAL_DP1_X=2560
NORMAL_DP1_Y=0
NORMAL_DP2_X=0
NORMAL_DP2_Y=0
NORMAL_DP3_X=7920
NORMAL_DP3_Y=0
NORMAL_HDMI_X=6000
NORMAL_HDMI_Y=180

# Gaming positions (adds large gaps to prevent cursor movement)
# DP-1 stays in same position, but DP-2 and DP-3 move far away
GAMING_DP1_X=2560
GAMING_DP1_Y=0
GAMING_DP2_X=-10000   # Move left monitor far to the left
GAMING_DP2_Y=0
GAMING_DP3_X=20000    # Move right monitor far to the right
GAMING_DP3_Y=0

# Check if wlr-randr is available
if ! command -v wlr-randr &> /dev/null; then
    notify-send "Gaming Mode" "wlr-randr not found. Please install wlr-randr to use this feature." -u critical
    exit 1
fi

if [ -f "$STATE_FILE" ]; then
    # Gaming mode is ON, switch to NORMAL
    echo "Switching to NORMAL mode - monitors adjacent"
    wlr-randr --output DP-2 --pos ${NORMAL_DP2_X},${NORMAL_DP2_Y} 2>/dev/null
    wlr-randr --output DP-1 --pos ${NORMAL_DP1_X},${NORMAL_DP1_Y} 2>/dev/null
    wlr-randr --output HDMI-A-1 --pos ${NORMAL_HDMI_X},${NORMAL_HDMI_Y} 2>/dev/null
    wlr-randr --output DP-3 --pos ${NORMAL_DP3_X},${NORMAL_DP3_Y} 2>/dev/null
    rm "$STATE_FILE"
    notify-send "Gaming Mode OFF" "Monitors restored to normal positions" -i input-gaming 2>/dev/null || notify-send "Gaming Mode OFF" "Monitors restored to normal positions"
else
    # Gaming mode is OFF, switch to GAMING
    echo "Switching to GAMING mode - cursor trapped on main monitor"
    wlr-randr --output DP-1 --pos ${GAMING_DP1_X},${GAMING_DP1_Y} 2>/dev/null
    wlr-randr --output DP-2 --pos ${GAMING_DP2_X},${GAMING_DP2_Y} 2>/dev/null
    wlr-randr --output DP-3 --pos ${GAMING_DP3_X},${GAMING_DP3_Y} 2>/dev/null
    touch "$STATE_FILE"
    notify-send "Gaming Mode ON" "Cursor confined to main monitor (DP-1)" -i input-gaming 2>/dev/null || notify-send "Gaming Mode ON" "Cursor confined to main monitor (DP-1)"
fi















