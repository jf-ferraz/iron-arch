#!/bin/bash
# Generate terminal colors from wallpaper using matugen

BACKGROUNDS_DIR="$HOME/.config/mango/backgrounds"

# Find the first image in the backgrounds folder
WALLPAPER=$(find "$BACKGROUNDS_DIR" -type f \( -iname "*.jpg" -o -iname "*.jpeg" -o -iname "*.png" -o -iname "*.webp" \) 2>/dev/null | sort | head -1)

if [ -z "$WALLPAPER" ] || [ ! -f "$WALLPAPER" ]; then
    echo "No wallpaper found in $BACKGROUNDS_DIR"
    echo "Please add an image file to that folder"
    exit 1
fi

# Generate colors and extract JSON (ignore warning)
echo "Generating colors from wallpaper..."
# Using scheme-content for content-aware color matching
COLORS_JSON=$(matugen image "$WALLPAPER" -m dark --type scheme-content -j hex 2>&1 | grep -v "image format")

# Extract key colors using simple text parsing (since jq might not be available)
BG=$(echo "$COLORS_JSON" | grep -o '"background".*"dark": *"[^"]*"' | grep -o '#[^"]*' | head -1)
FG=$(echo "$COLORS_JSON" | grep -o '"on_background".*"dark": *"[^"]*"' | grep -o '#[^"]*' | head -1)
PRIMARY=$(echo "$COLORS_JSON" | grep -o '"primary".*"dark": *"[^"]*"' | grep -o '#[^"]*' | head -1)

# Alternative method: use python if available
if command -v python3 &> /dev/null; then
    BG=$(echo "$COLORS_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin)['colors']['background']['dark'])" 2>/dev/null)
    FG=$(echo "$COLORS_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin)['colors']['on_background']['dark'])" 2>/dev/null)
    PRIMARY=$(echo "$COLORS_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin)['colors']['primary']['dark'])" 2>/dev/null)
fi

if [ -z "$BG" ]; then
    echo "Error: Could not extract colors"
    echo "Raw output:"
    echo "$COLORS_JSON" | head -20
    exit 1
fi

# Convert to hex without # for terminals
BG_HEX=$(echo "$BG" | tr -d '#')
FG_HEX=$(echo "$FG" | tr -d '#')
PRIMARY_HEX=$(echo "$PRIMARY" | tr -d '#')

echo ""
echo "✓ Colors generated successfully!"
echo ""
echo "Generated colors (dark mode):"
echo "Background: $BG ($BG_HEX)"
echo "Foreground: $FG ($FG_HEX)"
echo "Primary:    $PRIMARY ($PRIMARY_HEX)"
echo ""
echo "To apply these colors:"
echo "1. For foot: Update ~/.config/mango/foot/foot.ini"
echo "   - background=$BG_HEX"
echo "   - foreground=$FG_HEX"
echo ""
echo "2. For kitty: Update ~/.config/kitty/theme.conf"
echo "   - background $BG"
echo "   - foreground $FG"
echo ""
echo "Full color JSON saved to: ~/.cache/matugen/colors.json (if generated)"
