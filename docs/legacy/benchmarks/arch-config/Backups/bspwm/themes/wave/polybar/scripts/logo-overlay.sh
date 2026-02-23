#!/bin/bash

# Kill any existing logo overlay
pkill -f "feh.*logo-small.png"

# Get screen resolution and polybar position
# Polybar is at bottom with height 38, logo should be at bottom-left
LOGO_SIZE=32
LOGO_X=10
LOGO_Y=$(xrandr | grep '*' | head -n1 | awk '{print $1}' | cut -d'x' -f2)
LOGO_Y=$((LOGO_Y - 38 + 3))  # 38 is polybar height, +3 for padding

# Display logo using feh with borderless window
feh --borderless \
    --geometry ${LOGO_SIZE}x${LOGO_SIZE}+${LOGO_X}+${LOGO_Y} \
    --class "polybar-logo" \
    ~/.config/bspwm/themes/wave/polybar/logo-small.png &

# Make the window click-through and always on top
sleep 0.2
LOGO_WIN=$(xdotool search --class "polybar-logo" | tail -1)
if [ -n "$LOGO_WIN" ]; then
    xdotool windowraise $LOGO_WIN
    # Add click handler
    xdotool behave $LOGO_WIN mouse-click exec ~/.config/bspwm/scripts/rofi_launcher
fi
