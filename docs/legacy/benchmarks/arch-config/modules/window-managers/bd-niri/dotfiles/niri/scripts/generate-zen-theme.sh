#!/bin/bash
# Generate Zen Browser theme from Noctalia colors

NOCTALIA_COLORS="$HOME/.config/noctalia/colors.json"
ZEN_PROFILE="$HOME/.zen/frxlg5v7.Default (release)"
ZEN_CHROME="$ZEN_PROFILE/chrome"

if [ ! -f "$NOCTALIA_COLORS" ]; then
    echo "Error: Noctalia colors.json not found"
    exit 1
fi

if [ ! -d "$ZEN_CHROME" ]; then
    echo "Error: Zen Browser chrome directory not found"
    exit 1
fi

# Extract colors
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
HOVER=$(grep -o '"mHover": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')
ON_HOVER=$(grep -o '"mOnHover": *"[^"]*"' "$NOCTALIA_COLORS" | grep -o '#[^"]*')

# Generate userChrome.css
cat > "$ZEN_CHROME/userChrome.css" << EOF
/* Zen Browser - Noctalia Material Design Colors */
/* Auto-generated from ~/.config/noctalia/colors.json */

* {
  --primary:        $PRIMARY;
  --on-primary:     $ON_PRIMARY;
  --secondary:      $SECONDARY;
  --on-secondary:   $ON_SECONDARY;
  --tertiary:       $TERTIARY;
  --error:          $ERROR;
  --surface:        $SURFACE;
  --on-surface:     $ON_SURFACE;
  --surface-variant: $SURFACE_VARIANT;
  --on-surface-variant: $ON_SURFACE_VARIANT;
  --outline:        $OUTLINE;
  --hover:          $HOVER;
  --on-hover:       $ON_HOVER;
}

:root {
  /* Core Color Palette */
  --zen-colors-primary: var(--surface) !important;
  --zen-primary-color: var(--primary) !important;
  --zen-colors-secondary: var(--surface-variant) !important;
  --zen-colors-tertiary: var(--surface-variant) !important;
  --zen-colors-border: var(--primary) !important;

  /* Backgrounds */
  --toolbar-bgcolor: var(--surface) !important;
  --newtab-background-color: var(--surface) !important;
  --zen-themed-toolbar-bg: var(--surface) !important;
  --zen-main-browser-background: var(--surface) !important;
  --toolbox-bgcolor-inactive: var(--surface) !important;
  --arrowpanel-background: var(--surface-variant) !important;

  /* Text & Icons */
  --lwt-text-color: var(--on-surface) !important;
  --toolbarbutton-icon-fill: var(--primary) !important;
  --toolbar-field-color: var(--on-surface) !important;
  --toolbar-field-focus-color: var(--on-surface) !important;
  --toolbar-color: var(--on-surface) !important;
  --newtab-text-primary-color: var(--on-surface) !important;
  --arrowpanel-color: var(--on-surface) !important;

  /* Sidebar */
  --sidebar-text-color: var(--on-surface) !important;
  --lwt-sidebar-text-color: var(--on-surface) !important;
  --lwt-sidebar-background-color: var(--surface) !important;

  /* Tabs */
  --tab-selected-textcolor: var(--primary) !important;
  --tab-selected-bgcolor: var(--on-primary) !important;
  
  /* Buttons */
  --toolbarbutton-hover-background: var(--hover) !important;
  --button-primary-bgcolor: var(--primary) !important;
  --button-primary-color: var(--on-primary) !important;
  
  /* Input Fields */
  --toolbar-field-background-color: var(--surface-variant) !important;
  --toolbar-field-border-color: var(--outline) !important;
}
EOF

# Generate userContent.css
cat > "$ZEN_CHROME/userContent.css" << EOF
/* Zen Browser Content - Noctalia Colors */
/* Auto-generated from ~/.config/noctalia/colors.json */

@-moz-document url-prefix("about:") {
  :root {
    --in-content-page-background: $SURFACE !important;
    --in-content-page-color: $ON_SURFACE !important;
    --in-content-primary-button-background: $PRIMARY !important;
    --in-content-primary-button-background-hover: $SECONDARY !important;
  }
}
EOF

echo "✓ Zen Browser theme generated successfully!"
echo "Colors used:"
echo "  Primary: $PRIMARY"
echo "  Surface: $SURFACE"
echo "  On Surface: $ON_SURFACE"
echo ""
echo "Restart Zen Browser to see changes"
