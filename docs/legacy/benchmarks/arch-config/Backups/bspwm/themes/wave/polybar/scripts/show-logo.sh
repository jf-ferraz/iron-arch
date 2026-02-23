#!/bin/bash

# Script to display logo image overlaid on polybar using feh

LOGO_PATH="/home/don/.config/arch-config/logos/blackdontrans.png"
CONFIG_DIR="$HOME/.config/bspwm/themes/wave/polybar"
LOGO_SMALL="$CONFIG_DIR/logo-small.png"

# Kill existing instance
pkill -f "feh.*logo-small"

# Ensure small logo exists
if [ ! -f "$LOGO_SMALL" ]; then
    magick "$LOGO_PATH" -resize 28x28 "$LOGO_SMALL" 2>/dev/null
fi

# Get primary monitor resolution
SCREEN_HEIGHT=$(xdotool getdisplaygeometry | awk '{print $2}')

# Calculate position (polybar is at bottom with height 38)
POLYBAR_HEIGHT=38
LOGO_SIZE=28
Y_POS=$((SCREEN_HEIGHT - POLYBAR_HEIGHT + 5))
X_POS=12

# Display logo using feh with proper options to avoid tiling
feh --borderless \
    --scale-down \
    --auto-zoom \
    --geometry ${LOGO_SIZE}x${LOGO_SIZE}+${X_POS}+${Y_POS} \
    --title "polybar-logo" \
    --class "polybar-logo" \
    --image-bg black \
    "$LOGO_SMALL" &

# Wait for window to appear and configure it
sleep 0.3
LOGO_WIN=$(xdotool search --class "polybar-logo" | tail -1)
if [ -n "$LOGO_WIN" ]; then
    # Keep window on top and make it sticky
    xdotool windowraise "$LOGO_WIN"
    wmctrl -i -r "$LOGO_WIN" -b add,sticky,above
fi
