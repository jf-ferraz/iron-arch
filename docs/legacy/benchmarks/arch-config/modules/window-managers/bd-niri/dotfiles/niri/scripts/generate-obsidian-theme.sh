#!/bin/bash
# Generate Obsidian CSS snippet from Noctalia colors

NOCTALIA_COLORS="$HOME/.config/noctalia/colors.json"
VAULT_PATH="$HOME/Documents/notes-synced"
SNIPPETS_DIR="$VAULT_PATH/.obsidian/snippets"
SNIPPET_FILE="$SNIPPETS_DIR/noctalia.css"

if [ ! -f "$NOCTALIA_COLORS" ]; then
    echo "Error: Noctalia colors.json not found"
    exit 1
fi

if [ ! -d "$VAULT_PATH" ]; then
    echo "Error: Obsidian vault not found at $VAULT_PATH"
    exit 1
fi

# Create snippets directory if it doesn't exist
mkdir -p "$SNIPPETS_DIR"

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

# Generate CSS snippet
cat > "$SNIPPET_FILE" << EOF
/* Noctalia Theme for Obsidian */
/* Auto-generated from ~/.config/noctalia/colors.json */

.theme-dark {
  /* Base colors */
  --background-primary: $SURFACE;
  --background-primary-alt: $SURFACE_VARIANT;
  --background-secondary: $SURFACE_VARIANT;
  --background-secondary-alt: $SURFACE;
  --background-modifier-border: $OUTLINE;
  
  /* Text colors */
  --text-normal: $ON_SURFACE;
  --text-muted: $ON_SURFACE_VARIANT;
  --text-faint: $OUTLINE;
  --text-accent: $PRIMARY;
  --text-accent-hover: $SECONDARY;
  
  /* Interactive elements */
  --interactive-normal: $SURFACE_VARIANT;
  --interactive-hover: $ON_PRIMARY;
  --interactive-accent: $PRIMARY;
  --interactive-accent-hover: $SECONDARY;
  
  /* Links */
  --link-color: $PRIMARY;
  --link-color-hover: $SECONDARY;
  --link-external-color: $TERTIARY;
  
  /* Tags */
  --tag-background: $ON_PRIMARY;
  --tag-background-hover: $ON_SECONDARY;
  
  /* Code blocks */
  --code-normal: $ON_SURFACE;
  --code-background: $SURFACE_VARIANT;
  
  /* Headers */
  --h1-color: $PRIMARY;
  --h2-color: $PRIMARY;
  --h3-color: $SECONDARY;
  --h4-color: $SECONDARY;
  --h5-color: $TERTIARY;
  --h6-color: $TERTIARY;
  
  /* Scrollbars */
  --scrollbar-bg: $SURFACE;
  --scrollbar-thumb-bg: $OUTLINE;
  --scrollbar-active-thumb-bg: $PRIMARY;
  
  /* Dividers */
  --hr-color: $OUTLINE;
  
  /* Graph view */
  --graph-line: $OUTLINE;
  --graph-node: $PRIMARY;
  --graph-node-unresolved: $ON_SURFACE_VARIANT;
  --graph-node-tag: $TERTIARY;
  --graph-node-attachment: $SECONDARY;
}

/* Active line highlighting */
.cm-active.cm-line {
  background-color: $SURFACE_VARIANT;
}

/* Selection */
::selection {
  background-color: $ON_PRIMARY;
  color: $ON_SURFACE;
}

/* Checkbox styling */
input[type="checkbox"]:checked {
  background-color: $PRIMARY;
  border-color: $PRIMARY;
}

/* Button styling */
.mod-cta {
  background-color: $PRIMARY;
  color: $ON_PRIMARY;
}

.mod-cta:hover {
  background-color: $SECONDARY;
}

/* Sidebar */
.nav-file-title,
.nav-folder-title {
  color: $ON_SURFACE;
}

.nav-file-title.is-active,
.nav-folder-title.is-active {
  background-color: $ON_PRIMARY;
  color: $PRIMARY;
}

/* Status bar */
.status-bar {
  background-color: $SURFACE_VARIANT;
  color: $ON_SURFACE_VARIANT;
}
EOF

echo "✓ Obsidian theme snippet generated successfully!"
echo "Location: $SNIPPET_FILE"
echo ""
echo "Colors used:"
echo "  Primary: $PRIMARY"
echo "  Surface: $SURFACE"
echo "  On Surface: $ON_SURFACE"
echo ""
echo "To enable the theme:"
echo "  1. Open Obsidian Settings (Ctrl+,)"
echo "  2. Go to Appearance → CSS snippets"
echo "  3. Enable 'noctalia'"
echo "  4. The theme will apply immediately"
