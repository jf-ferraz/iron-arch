#!/bin/bash
# Generate qutebrowser theme from Noctalia colors
# This script updates qutebrowser colors when wallpaper changes

NOCTALIA_COLORS="$HOME/.config/noctalia/colors.json"
QUTEBROWSER_CONFIG="$HOME/.config/qutebrowser/config.py"

if [ ! -f "$NOCTALIA_COLORS" ]; then
    echo "Error: Noctalia colors.json not found at $NOCTALIA_COLORS"
    exit 1
fi

if [ ! -f "$QUTEBROWSER_CONFIG" ]; then
    echo "Error: qutebrowser config.py not found at $QUTEBROWSER_CONFIG"
    exit 1
fi

# Extract colors from noctalia colors.json
PRIMARY=$(grep -o '"mPrimary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_PRIMARY=$(grep -o '"mOnPrimary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
SECONDARY=$(grep -o '"mSecondary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_SECONDARY=$(grep -o '"mOnSecondary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
TERTIARY=$(grep -o '"mTertiary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_TERTIARY=$(grep -o '"mOnTertiary": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ERROR=$(grep -o '"mError": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_ERROR=$(grep -o '"mOnError": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
SURFACE=$(grep -o '"mSurface": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_SURFACE=$(grep -o '"mOnSurface": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
SURFACE_VARIANT=$(grep -o '"mSurfaceVariant": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_SURFACE_VARIANT=$(grep -o '"mOnSurfaceVariant": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
OUTLINE=$(grep -o '"mOutline": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')

echo "Updating qutebrowser theme with Noctalia colors..."
echo "  Primary: $PRIMARY"
echo "  Surface: $SURFACE"
echo "  On Surface: $ON_SURFACE"

# Check if qutebrowser is running and reload config
if pgrep -x "qutebrowser" > /dev/null; then
    # Send reload command to qutebrowser
    qutebrowser ":config-source" 2>/dev/null || true
    echo "✓ qutebrowser config reloaded"
fi

echo "✓ qutebrowser theme updated successfully!"
