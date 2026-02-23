#!/bin/bash
mkdir -p ~/Pictures/Screenshots
FILENAME="$HOME/Pictures/Screenshots/Screenshot-$(date +%Y-%m-%d-%H-%M-%S).png"
grim -g "$(slurp -b '#2E2A1E55' -c '#fb751bff')" "$FILENAME"
wl-copy < "$FILENAME"
notify-send "Screenshot" "Saved and copied: $FILENAME"

