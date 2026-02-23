#!/bin/bash
# Generate colors from wallpaper and apply to terminals using pywal

BACKGROUNDS_DIR="$HOME/.config/mango/backgrounds"
FOOT_CONFIG="$HOME/.config/mango/foot/foot.ini"
KITTY_THEME="$HOME/.config/kitty/theme.conf"
HTOP_SCRIPT="$HOME/.config/wal/templates/htoprc.sh"
BTOP_THEME="$HOME/.config/btop/themes/pywal.theme"
BTOP_CONFIG="$HOME/.config/btop/btop.conf"

# Find the first image in the backgrounds folder
WALLPAPER=$(find "$BACKGROUNDS_DIR" -type f \( -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.png" -o -iname "*.webp" \) 2>/dev/null | sort | head -1)

if [ -z "$WALLPAPER" ] || [ ! -f "$WALLPAPER" ]; then
    notify-send "Wallpaper Colors" "No wallpaper found in $BACKGROUNDS_DIR\nPlease add an image file to that folder" -u critical
    exit 1
fi

# Generate colors using pywal
# Flags: -i (input image), -n (skip wallpaper setting), -q (quiet mode), -o (post-execution script)
# The -o flag runs htoprc.sh after generating colors to update htop config
wal -i "$WALLPAPER" -n -q -o "$HTOP_SCRIPT" 2>&1 > /dev/null

if [ $? -ne 0 ]; then
    notify-send "Wallpaper Colors" "Failed to generate colors with pywal" -u critical
    exit 1
fi

# Read colors from pywal's generated colors file
if [ ! -f "$HOME/.cache/wal/colors.json" ]; then
    notify-send "Wallpaper Colors" "Pywal colors file not found" -u critical
    exit 1
fi

# Extract colors from pywal's JSON output
BG=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['special']['background'])" 2>/dev/null)
FG=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['special']['foreground'])" 2>/dev/null)
PRIMARY=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color4'])" 2>/dev/null)
SECONDARY=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color6'])" 2>/dev/null)

# Extract all 16 colors for btop
COLOR0=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color0'])" 2>/dev/null)
COLOR1=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color1'])" 2>/dev/null)
COLOR2=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color2'])" 2>/dev/null)
COLOR3=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color3'])" 2>/dev/null)
COLOR5=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color5'])" 2>/dev/null)
COLOR7=$(python3 -c "import json; print(json.load(open('$HOME/.cache/wal/colors.json'))['colors']['color7'])" 2>/dev/null)

if [ -z "$BG" ] || [ -z "$FG" ]; then
    notify-send "Wallpaper Colors" "Failed to extract colors from pywal output" -u critical
    exit 1
fi

# Convert to hex without # for foot
BG_HEX=$(echo "$BG" | tr -d '#' | tr '[:upper:]' '[:lower:]')
FG_HEX=$(echo "$FG" | tr -d '#' | tr '[:upper:]' '[:lower:]')

# Update foot config
if [ -f "$FOOT_CONFIG" ]; then
    # Create backup
    cp "$FOOT_CONFIG" "$FOOT_CONFIG.bak"

    # Update background and foreground in foot.ini
    sed -i "s/^background=.*/background=$BG_HEX/" "$FOOT_CONFIG"
    sed -i "s/^foreground=.*/foreground=$FG_HEX/" "$FOOT_CONFIG"

    echo "✓ Updated foot colors"
fi

# Update kitty theme
if [ -f "$KITTY_THEME" ]; then
    # Create backup
    cp "$KITTY_THEME" "$KITTY_THEME.bak"

    # Update background and foreground in kitty theme
    sed -i "s|^background[[:space:]]\+.*|background              $BG|" "$KITTY_THEME"
    sed -i "s|^foreground[[:space:]]\+.*|foreground              $FG|" "$KITTY_THEME"

    echo "✓ Updated kitty colors"
fi

# Update btop theme
if [ -d "$(dirname "$BTOP_THEME")" ]; then
    # Create a custom btop theme from pywal colors
    cat > "$BTOP_THEME" << EOF
# Pywal generated theme for btop
# Main background, empty for terminal default
theme[main_bg]="$BG"

# Main text color
theme[main_fg]="$FG"

# Title color for boxes
theme[title]="$FG"

# Highlight color for keyboard shortcuts
theme[hi_fg]="$PRIMARY"

# Background color of selected item in processes box
theme[selected_bg]="$PRIMARY"

# Foreground color of selected item in processes box
theme[selected_fg]="$BG"

# Color of inactive/disabled text
theme[inactive_fg]="$COLOR0"

# Color of text appearing on top of graphs
theme[graph_text]="$FG"

# Background color of the percentage meters
theme[meter_bg]="$COLOR0"

# Misc colors for processes box
theme[proc_misc]="$SECONDARY"

# Cpu box outline color
theme[cpu_box]="$PRIMARY"

# Memory/disks box outline color
theme[mem_box]="$COLOR2"

# Net up/down box outline color
theme[net_box]="$COLOR1"

# Processes box outline color
theme[proc_box]="$SECONDARY"

# Box divider line and small boxes line color
theme[div_line]="$COLOR0"

# Temperature graph colors
theme[temp_start]="$COLOR2"
theme[temp_mid]="$COLOR3"
theme[temp_end]="$COLOR1"

# CPU graph colors
theme[cpu_start]="$PRIMARY"
theme[cpu_mid]="$SECONDARY"
theme[cpu_end]="$COLOR2"

# Mem/Disk free meter
theme[free_start]="$COLOR5"
theme[free_mid]="$PRIMARY"
theme[free_end]="$SECONDARY"

# Mem/Disk cached meter
theme[cached_start]="$SECONDARY"
theme[cached_mid]="$PRIMARY"
theme[cached_end]="$COLOR2"

# Mem/Disk available meter
theme[available_start]="$COLOR3"
theme[available_mid]="$COLOR2"
theme[available_end]="$PRIMARY"

# Mem/Disk used meter
theme[used_start]="$COLOR2"
theme[used_mid]="$PRIMARY"
theme[used_end]="$SECONDARY"

# Download graph colors
theme[download_start]="$PRIMARY"
theme[download_mid]="$COLOR2"
theme[download_end]="$SECONDARY"

# Upload graph colors
theme[upload_start]="$COLOR5"
theme[upload_mid]="$PRIMARY"
theme[upload_end]="$COLOR1"

# Process box color gradient
theme[process_start]="$COLOR2"
theme[process_mid]="$PRIMARY"
theme[process_end]="$SECONDARY"
EOF

    # Update btop config to use the pywal theme
    if [ -f "$BTOP_CONFIG" ]; then
        sed -i 's/^color_theme = .*/color_theme = "pywal"/' "$BTOP_CONFIG"
    fi

    echo "✓ Updated btop colors"
fi

# Reload terminals if possible
if command -v footclient &> /dev/null; then
    # Reload foot terminals
    pkill -USR1 foot 2>/dev/null || true
fi

if command -v kitty &> /dev/null; then
    # Reload kitty terminals (requires kitty remote control)
    kitty @ set-colors --all "background=$BG" "foreground=$FG" 2>/dev/null || true
fi

notify-send "Wallpaper Colors" "Colors updated from wallpaper using pywal!\nTerminals: foot, kitty, htop, btop\nBG: $BG\nFG: $FG" -t 3000
