#!/bin/bash
# Screenshot script for grimblast with save to Pictures/Screenshots/

SCREENSHOT_DIR="$HOME/Pictures/Screenshots"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
FILENAME="screenshot_${TIMESTAMP}.png"

# Ensure directory exists
mkdir -p "$SCREENSHOT_DIR"

# Take screenshot and save
grimblast --freeze copysave area "${SCREENSHOT_DIR}/${FILENAME}"
