#!/bin/bash
# Generate btop theme from Noctalia colors

NOCTALIA_COLORS="$HOME/.config/noctalia/colors.json"
BTOP_THEME="$HOME/.config/btop/themes/matugen.theme"

if [ ! -f "$NOCTALIA_COLORS" ]; then
    echo "Error: Noctalia colors.json not found"
    exit 1
fi

# Extract colors using grep/sed (no jq dependency)
PRIMARY=$(grep -o '"mPrimary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_PRIMARY=$(grep -o '"mOnPrimary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
SECONDARY=$(grep -o '"mSecondary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_SECONDARY=$(grep -o '"mOnSecondary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
TERTIARY=$(grep -o '"mTertiary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ERROR=$(grep -o '"mError": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
SURFACE=$(grep -o '"mSurface": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_SURFACE=$(grep -o '"mOnSurface": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
SURFACE_VARIANT=$(grep -o '"mSurfaceVariant": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_SURFACE_VARIANT=$(grep -o '"mOnSurfaceVariant": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
OUTLINE=$(grep -o '"mOutline": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')

# Generate theme file
cat > "$BTOP_THEME" << EOF
# btop theme using Noctalia/matugen colors
# Auto-generated from ~/.config/noctalia/colors.json

# Main background
theme[main_bg]="$SURFACE"

# Main text color
theme[main_fg]="$ON_SURFACE"

# Title color for boxes
theme[title]="$PRIMARY"

# Highlight color for keyboard shortcuts
theme[hi_fg]="$PRIMARY"

# Background color of selected item in processes box
theme[selected_bg]="$ON_PRIMARY"

# Foreground color of selected item in processes box
theme[selected_fg]="$ON_SURFACE"

# Color of inactive/disabled text
theme[inactive_fg]="$ON_SURFACE_VARIANT"

# Misc colors for processes box
theme[proc_misc]="$SECONDARY"

# Box outline colors
theme[cpu_box]="$OUTLINE"
theme[mem_box]="$OUTLINE"
theme[net_box]="$OUTLINE"
theme[proc_box]="$OUTLINE"

# Box divider line and small boxes line color
theme[div_line]="$OUTLINE"

# Temperature graph colors
theme[temp_start]="$SECONDARY"
theme[temp_mid]="$PRIMARY"
theme[temp_end]="$ERROR"

# CPU graph colors
theme[cpu_start]="$SECONDARY"
theme[cpu_mid]="$PRIMARY"
theme[cpu_end]="$TERTIARY"

# Mem/Disk free meter
theme[free_start]="$SECONDARY"
theme[free_mid]="$PRIMARY"
theme[free_end]="$TERTIARY"

# Mem/Disk cached meter
theme[cached_start]="$SECONDARY"
theme[cached_mid]="$PRIMARY"
theme[cached_end]="$TERTIARY"

# Mem/Disk available meter
theme[available_start]="$SECONDARY"
theme[available_mid]="$PRIMARY"
theme[available_end]="$TERTIARY"

# Mem/Disk used meter
theme[used_start]="$PRIMARY"
theme[used_mid]="$ON_PRIMARY"
theme[used_end]="$ON_SECONDARY"

# Download graph colors
theme[download_start]="$SECONDARY"
theme[download_mid]="$PRIMARY"
theme[download_end]="$ON_PRIMARY"

# Upload graph colors
theme[upload_start]="$TERTIARY"
theme[upload_mid]="$PRIMARY"
theme[upload_end]="$ON_PRIMARY"
EOF

echo "✓ btop theme generated successfully!"
echo "Colors used:"
echo "  Primary: $PRIMARY"
echo "  Surface: $SURFACE"
echo "  On Surface: $ON_SURFACE"
