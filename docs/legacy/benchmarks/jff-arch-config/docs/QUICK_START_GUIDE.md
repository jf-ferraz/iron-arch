# Quick Start Guide - Enhanced TUI

## Getting Started

### Launch the TUI
```bash
cd /home/laraj/Documents/jff-arch-config
cargo run -p app-tui --release -- --root .
# Or if binaries are in PATH:
app-tui --root .
```

## Daily Workflows

### 1. Switch Shell Theme

**Keyboard Navigation**:
```
Press '3' → Navigate to Themes tab
↑↓ → Select desired theme
Enter → Switch to theme
'y' → Confirm
```

**Available Themes**:
- **dank** - Dark with neon accents (cyberpunk)
- **noctalia** - Midnight blue with gradients
- **minimal** - Clean monochrome
- **default** - Standard bash theme

### 2. Manage Modules

**Keyboard Navigation**:
```
Press '2' → Navigate to Modules tab
↑↓ → Navigate through modules
Space → Toggle checkbox [✓] / [ ]
Enter → Apply changes
'y' → Confirm
```

**Status Bar Shows**: "Modules (X/Y active)"

### 3. System Maintenance

**Keyboard Navigation**:
```
Press '1' → Navigate to System tab
↑↓ → Select operation
Enter → Execute
'y' → Confirm (if needed)
```

**Operations with Status Indicators**:
- 🔍 **System Doctor** - Health check (🟢 2h ago)
- 📊 **System Status** - Current status
- ⬆️ **System Update** - Package updates (🟡 3d ago)
- 🧹 **System Clean** - Clean cache (🔴 10d ago)

**Status Colors**:
- 🟢 **Green** - Recently run (< 1 day)
- 🟡 **Yellow** - Needs attention (1-7 days)
- 🔴 **Red** - Overdue (> 7 days)
- ⚪ **White** - Never run

## Keyboard Shortcuts

### Global Navigation
- `1-6` - Jump directly to tab
- `←→` - Move between tabs
- `↑↓` - Navigate items within tab
- `Enter` - Execute/Apply
- `r` - Refresh data
- `q` - Quit

### Tab-Specific
- **Modules Tab**: `Space` - Toggle checkbox
- **All Tabs**: `Enter` - Execute action/apply changes

## CLI Commands

### Theme Management
```bash
# List available themes
app-cli theme list --root .

# Show current theme
app-cli theme current --root .

# Switch theme
app-cli theme switch dank --root .
```

### System Status
```bash
# Repository status
app-cli status --root .

# Health check
app-cli doctor --root .

# Validation
app-cli validate --root .
```

## State Files

All state is stored in `app/state/tracking/`:
- `active_theme.json` - Current shell theme
- `active_modules.json` - Enabled modules
- `maintenance_state.json` - Maintenance history
- `hook_hashes.json` - Hook execution tracking
- `module_states/` - Per-module execution states

## Tips & Tricks

### Safe Exploration
- All preview operations (`[?]`) are safe to run
- Destructive operations (`[!]`) require confirmation
- Informational operations (`[i]`) never modify system

### Visual Indicators
- `[?]` **Green** - Safe preview/dry-run
- `[!]` **Yellow** - Requires confirmation
- `[i]` **White** - Informational only

### Maintenance Best Practices
1. Run **Doctor** weekly (shows 🟢 if recent)
2. Run **Update** when 🟡 or 🔴
3. Run **Clean** monthly to free space

### Module Management
- Checkboxes show current state
- Changes apply only after pressing Enter
- Failed operations show in status line
- Refresh (press `r`) to reload state

## Troubleshooting

### Theme not switching?
```bash
# Check theme list
app-cli theme list --root .

# Verify theme exists
cat app/manifests/themes.toml

# Check state file
cat app/state/tracking/active_theme.json
```

### Module toggle not working?
```bash
# Check active modules
cat app/state/tracking/active_modules.json

# Verify module exists
app-cli plan --root .

# Check module logs
cat app/state/logs/operations.jsonl | tail
```

### Status indicators not updating?
- Press `r` to refresh
- Check maintenance state: `cat app/state/tracking/maintenance_state.json`

## Advanced Usage

### Profile Workflows (Future)
Save common module configurations as profiles:
- **Gaming**: Enable performance modules
- **Work**: Enable productivity modules
- **Minimal**: Disable non-essential modules

### Hook Management (Future)
```bash
# List tracked hooks
app-cli hooks list --root .

# Set hook behavior
app-cli hooks set-behavior <hook-id> once

# Reset hook execution
app-cli hooks reset <hook-id>
```

## Getting Help

- Press `?` or `h` in TUI for help (if implemented)
- Read the implementation summary: `docs/IMPLEMENTATION_SUMMARY.md`
- Check the main plan: (previous conversation context)

## What's Next?

This enhanced TUI makes daily Arch management simple and visual. Key improvements:

✅ **No Terminal Commands** - Everything in the TUI
✅ **Visual Status** - See maintenance status at a glance
✅ **Simple Toggles** - Check/uncheck to enable modules
✅ **Theme Switching** - One-click shell theme changes
✅ **Smart Backend** - Automatic optimization and tracking

**Enjoy your streamlined Arch Linux experience!** 🚀
