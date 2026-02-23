#!/usr/bin/env python3
# qutebrowser configuration with Noctalia theme
# Auto-generated and updated via noctalia theme system

import os
import json

# Load autoconfig (required in newer qutebrowser versions)
config.load_autoconfig()

# === Theme Configuration ===
# Colors are dynamically loaded from noctalia theme
# Path to noctalia colors
NOCTALIA_COLORS_PATH = os.path.expanduser("~/.config/noctalia/colors.json")

def load_noctalia_colors():
    """Load colors from noctalia theme file"""
    default_colors = {
        "mPrimary": "#c4c0ff",
        "mOnPrimary": "#2a2377",
        "mSecondary": "#c7c4dc",
        "mOnSecondary": "#302e42",
        "mTertiary": "#ebb9d0",
        "mOnTertiary": "#472638",
        "mError": "#ffb4ab",
        "mOnError": "#690005",
        "mSurface": "#131316",
        "mOnSurface": "#e5e1e6",
        "mSurfaceVariant": "#201f23",
        "mOnSurfaceVariant": "#c8c5d0",
        "mOutline": "#47464f",
        "mShadow": "#000000",
        "mHover": "#ebb9d0",
        "mOnHover": "#472638"
    }
    
    try:
        with open(NOCTALIA_COLORS_PATH, 'r') as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return default_colors

# Load colors
colors = load_noctalia_colors()

# === General Settings ===
# Dark mode preference - prefer dark based on system but don't force it
c.content.prefers_reduced_motion = True
c.colors.webpage.darkmode.enabled = False  # Don't force dark mode (let sites render naturally)
c.colors.webpage.preferred_color_scheme = "dark"  # Tell websites we prefer dark mode (matches system)
c.colors.webpage.darkmode.policy.images = 'never'
c.colors.webpage.darkmode.policy.page = 'smart'  # Smart detection

# === Fonts ===
c.fonts.default_family = 'JetBrainsMono Nerd Font, monospace'
c.fonts.default_size = '10pt'
c.fonts.web.family.standard = 'JetBrainsMono Nerd Font, sans-serif'
c.fonts.web.family.fixed = 'JetBrainsMono Nerd Font, monospace'
c.fonts.web.family.serif = 'JetBrainsMono Nerd Font, serif'
c.fonts.web.family.sans_serif = 'JetBrainsMono Nerd Font, sans-serif'

# === Tab Settings (Larger Tabs) ===
c.tabs.padding = {"top": 10, "bottom": 10, "left": 15, "right": 15}  # Larger padding for bigger tabs
c.tabs.indicator.width = 3
c.tabs.favicons.scale = 1.2  
c.tabs.title.alignment = 'center'
c.tabs.title.format = '{audio} {index}'
c.tabs.show = 'always'  # Always show tab bar
c.tabs.position = 'top'
c.tabs.tooltips = True
# Vertical Tabs Width
c.tabs.width = 100

# === Window Settings ===
c.window.transparent = True  # Enable transparency
c.window.title_format = '{perc}{current_title}{title_sep}qutebrowser'

# === Status Bar Settings ===
c.statusbar.padding = {"top": 6, "bottom": 6, "left": 8, "right": 8}
c.statusbar.show = 'in-mode'  # Show in command/mode line
c.statusbar.position = 'bottom'
c.statusbar.widgets = ['keypress', 'url', 'scroll', 'history', 'tabs', 'progress']

# === Download Settings ===
c.downloads.position = 'bottom'
c.downloads.remove_finished = 10000  # Remove finished downloads after 10s

# === Hint Settings ===
c.hints.mode = 'letter'
c.hints.chars = 'asdfghjklqwertyuiopzxcvbnm'
c.hints.uppercase = True
c.hints.auto_follow = 'unique-match'
c.hints.padding = {"top": 4, "bottom": 4, "left": 4, "right": 4}

# === Noctalia Color Scheme ===
# Background
bg = colors.get('mSurface', '#131316')
bg_variant = colors.get('mSurfaceVariant', '#201f23')

# Foreground
fg = colors.get('mOnSurface', '#e5e1e6')
fg_variant = colors.get('mOnSurfaceVariant', '#c8c5d0')

# Accents
primary = colors.get('mPrimary', '#c4c0ff')
primary_container = colors.get('mOnPrimary', '#2a2377')
secondary = colors.get('mSecondary', '#c7c4dc')
secondary_container = colors.get('mOnSecondary', '#302e42')
tertiary = colors.get('mTertiary', '#ebb9d0')
error = colors.get('mError', '#ffb4ab')
outline = colors.get('mOutline', '#47464f')

# === Color Configurations ===

# Completion/Menu colors
c.colors.completion.category.bg = primary
c.colors.completion.category.fg = primary_container
c.colors.completion.category.border.bottom = primary
c.colors.completion.category.border.top = primary
c.colors.completion.even.bg = bg_variant
c.colors.completion.odd.bg = bg
c.colors.completion.fg = fg
c.colors.completion.item.selected.bg = primary
c.colors.completion.item.selected.fg = primary_container
c.colors.completion.item.selected.border.bottom = primary
c.colors.completion.item.selected.border.top = primary
c.colors.completion.match.fg = tertiary
c.colors.completion.scrollbar.bg = bg
c.colors.completion.scrollbar.fg = fg_variant

# Completion window sizing
c.completion.height = '15%'  # Height of completion window
c.completion.scrollbar.width = 8 

# Context menu
c.colors.contextmenu.disabled.bg = bg_variant
c.colors.contextmenu.disabled.fg = fg_variant
c.colors.contextmenu.menu.bg = bg
c.colors.contextmenu.menu.fg = fg
c.colors.contextmenu.selected.bg = primary
c.colors.contextmenu.selected.fg = primary_container

# Downloads
c.colors.downloads.bar.bg = bg
c.colors.downloads.error.bg = error
c.colors.downloads.error.fg = colors.get('mOnError', '#690005')
c.colors.downloads.start.bg = primary
c.colors.downloads.start.fg = primary_container
c.colors.downloads.stop.bg = secondary
c.colors.downloads.stop.fg = secondary_container
c.colors.downloads.system.bg = 'rgb'
c.colors.downloads.system.fg = 'rgb'

# Hints
c.colors.hints.bg = primary
c.colors.hints.fg = primary_container
c.colors.hints.match.fg = tertiary

# Keyhints
c.colors.keyhint.bg = bg
c.colors.keyhint.fg = fg
c.colors.keyhint.suffix.fg = primary

# Messages/Info
c.colors.messages.error.bg = error
c.colors.messages.error.border = error
c.colors.messages.error.fg = colors.get('mOnError', '#690005')
c.colors.messages.info.bg = bg_variant
c.colors.messages.info.border = outline
c.colors.messages.info.fg = fg
c.colors.messages.warning.bg = tertiary
c.colors.messages.warning.border = tertiary
c.colors.messages.warning.fg = colors.get('mOnTertiary', '#472638')

# Prompts
c.colors.prompts.bg = bg_variant
c.colors.prompts.border = '1px solid ' + outline
c.colors.prompts.fg = fg
c.colors.prompts.selected.bg = primary
c.colors.prompts.selected.fg = primary_container

# Statusbar
c.colors.statusbar.caret.bg = tertiary
c.colors.statusbar.caret.fg = colors.get('mOnTertiary', '#472638')
c.colors.statusbar.caret.selection.bg = secondary
c.colors.statusbar.caret.selection.fg = secondary_container
c.colors.statusbar.command.bg = bg
c.colors.statusbar.command.fg = fg
c.colors.statusbar.command.private.bg = bg_variant
c.colors.statusbar.command.private.fg = fg_variant
c.colors.statusbar.insert.bg = primary
c.colors.statusbar.insert.fg = primary_container
c.colors.statusbar.normal.bg = bg
c.colors.statusbar.normal.fg = fg
c.colors.statusbar.passthrough.bg = secondary
c.colors.statusbar.passthrough.fg = secondary_container
c.colors.statusbar.private.bg = bg_variant
c.colors.statusbar.private.fg = fg_variant
c.colors.statusbar.progress.bg = primary
c.colors.statusbar.url.error.fg = error
c.colors.statusbar.url.fg = fg
c.colors.statusbar.url.hover.fg = tertiary
c.colors.statusbar.url.success.http.fg = fg_variant
c.colors.statusbar.url.success.https.fg = primary
c.colors.statusbar.url.warn.fg = tertiary

# Tabs
c.colors.tabs.bar.bg = bg
c.colors.tabs.even.bg = bg
c.colors.tabs.even.fg = fg_variant
c.colors.tabs.indicator.error = error
c.colors.tabs.indicator.start = primary
c.colors.tabs.indicator.stop = secondary
c.colors.tabs.indicator.system = 'rgb'
c.colors.tabs.odd.bg = bg_variant
c.colors.tabs.odd.fg = fg_variant
c.colors.tabs.pinned.even.bg = secondary
c.colors.tabs.pinned.even.fg = secondary_container
c.colors.tabs.pinned.odd.bg = secondary
c.colors.tabs.pinned.odd.fg = secondary_container
c.colors.tabs.pinned.selected.even.bg = primary
c.colors.tabs.pinned.selected.even.fg = primary_container
c.colors.tabs.pinned.selected.odd.bg = primary
c.colors.tabs.pinned.selected.odd.fg = primary_container
c.colors.tabs.selected.even.bg = primary
c.colors.tabs.selected.even.fg = primary_container
c.colors.tabs.selected.odd.bg = primary
c.colors.tabs.selected.odd.fg = primary_container

# Webpage
c.colors.webpage.bg = bg

# === Search Engines ===
c.url.searchengines = {
    'DEFAULT': 'https://duckduckgo.com/?q={}',
    'google': 'https://www.google.com/search?q={}',
    'yt': 'https://www.youtube.com/results?search_query={}',
}

# === Default Settings ===
c.url.default_page = 'https://duckduckgo.com'
c.url.start_pages = ['https://duckduckgo.com']

# === Content Settings ===
c.content.autoplay = False
c.content.canvas_reading = False
c.content.geolocation = False
c.content.headers.referer = 'same-domain'
c.content.blocking.enabled = True
c.content.blocking.method = 'auto'
c.content.javascript.enabled = True
c.content.notifications.enabled = False
c.content.webgl = True

# === Cloudflare/Security Bypass Settings ===
# Try to look more like Chrome to bypass Cloudflare detection
c.content.headers.user_agent = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'
c.content.headers.accept_language = 'en-US,en;q=0.9'
c.content.headers.custom = {"sec-ch-ua": "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"", "sec-ch-ua-platform": "\"Linux\"", "sec-ch-ua-mobile": "?0"}
c.content.canvas_reading = True
c.content.webgl = True
c.content.dns_prefetch = True

# Additional fingerprinting protection
c.content.javascript.log = {'unknown': 'none', 'error': 'none', 'info': 'none', 'warning': 'none'}

# === Performance Optimizations ===
c.qt.args = [
    '--disable-features=IsolateOrigins,site-per-process,OptimizationHints,NetworkPrediction,OfflinePagesPrefetching,Translate',
    '--disable-site-isolation-trials',
    '--transparent-background',
    '--disable-background-networking',  # Disable background networking for faster startup
    '--disable-sync',  # Disable sync services
    '--disable-default-apps',  # Skip loading default apps
    '--disable-hang-monitor',  # Disable hang monitoring
    '--disable-component-extensions-with-background-pages',  # Disable background component extensions
    '--aggressive-cache-discard',  # Aggressively discard cache
]

# === Privacy Settings ===
c.content.cookies.accept = 'all'
c.content.webrtc_ip_handling_policy = 'default-public-interface-only'

# === Editor ===
c.editor.command = ['kitty', '-e', 'helix', '{file}']

# === File Selectors ===
# Use built-in Qt file picker (most reliable)
c.fileselect.handler = 'default'

# === Zoom ===
c.zoom.default = '100%'
c.zoom.levels = ['25%', '33%', '50%', '67%', '75%', '90%', '100%', '110%', '125%', '150%', '175%', '200%', '250%', '300%', '400%', '500%']

# === Spell Check ===
c.spellcheck.languages = []  # Disable spellcheck to avoid dictionary installation requirement

# === Session ===
# Disable auto-save and restore to speed up startup
# You can manually save sessions with `:session-save` and restore with `:session-load`
c.auto_save.session = False
c.session.default_name = None

# === Confirm Quit ===
c.confirm_quit = ['downloads']

# === Keybindings ===
# Leader key is 'o' for "Open"

# Dark mode toggle (useful for sites like Canva where colors need to be accurate)
config.bind('<Ctrl+Shift+d>', 'config-cycle colors.webpage.darkmode.enabled')
# General navigation
config.bind('o', 'cmd-set-text -s :open ')
config.bind('O', 'cmd-set-text -s :open -t ')
config.bind('t', 'cmd-set-text -s :open -t ')
config.bind('T', 'cmd-set-text -s :open ')

# Tab management
config.bind('J', 'tab-prev')
config.bind('K', 'tab-next')
config.bind('gT', 'tab-prev')
config.bind('gt', 'tab-next')
config.bind('<Ctrl+Shift+{>', 'tab-prev')  # Ctrl+Shift+[
config.bind('<Ctrl+Shift+}>', 'tab-next')  # Ctrl+Shift+]
config.bind('x', 'tab-close')
config.bind('<Ctrl+Shift+w>', 'tab-close')  # Ctrl+Shift+W
config.bind('X', 'undo')
config.bind('d', 'tab-close')
config.bind('D', 'tab-close -o')
config.bind('<', 'tab-move -')
config.bind('>', 'tab-move +')
config.bind('p', 'tab-pin')  # Toggle pin/unpin tab
config.bind('<Ctrl+Shift+t>', 'config-cycle tabs.position top left right bottom')  # Cycle tab position

# Jump to tab by number (Alt+1 through Alt+9, Alt+0 for tab 10)
config.bind('<Alt+1>', 'tab-focus 1')
config.bind('<Alt+2>', 'tab-focus 2')
config.bind('<Alt+3>', 'tab-focus 3')
config.bind('<Alt+4>', 'tab-focus 4')
config.bind('<Alt+5>', 'tab-focus 5')
config.bind('<Alt+6>', 'tab-focus 6')
config.bind('<Alt+7>', 'tab-focus 7')
config.bind('<Alt+8>', 'tab-focus 8')
config.bind('<Alt+9>', 'tab-focus 9')
config.bind('<Alt+0>', 'tab-focus 10')

# Navigation
config.bind('h', 'scroll left')
config.bind('j', 'scroll down')
config.bind('k', 'scroll up')
config.bind('l', 'scroll right')
config.bind('gg', 'scroll-to-perc 0')
config.bind('G', 'scroll-to-perc 100')
config.bind('H', 'back')
config.bind('L', 'forward')
config.bind('<Alt+Left>', 'back')
config.bind('<Alt+Right>', 'forward')

# Reload
config.bind('r', 'reload')
config.bind('R', 'reload -f')

# Zoom
config.bind('zi', 'zoom-in')
config.bind('zo', 'zoom-out')
config.bind('zz', 'zoom-reset')
config.bind('<Ctrl+Shift++>', 'zoom-in')
config.bind('<Ctrl+Shift+_>', 'zoom-out')

# Search
config.bind('/', 'cmd-set-text /')
config.bind('?', 'cmd-set-text ?')
config.bind('n', 'search-next')
config.bind('N', 'search-prev')

# Hints
config.bind('f', 'hint')
config.bind('F', 'hint all tab')
config.bind(';o', 'hint links fill :open -t {hint-url}')
config.bind(';y', 'hint links yank')
config.bind(';Y', 'hint links yank-primary')

# Yank
config.bind('yy', 'yank')
config.bind('yY', 'yank -t')
config.bind('yd', 'yank domain')
config.bind('yD', 'yank -t domain')
config.bind('yp', 'yank pretty-url')
config.bind('yP', 'yank -t pretty-url')
config.bind('yt', 'yank title')
config.bind('yT', 'yank -t title')

# Quick bookmarks
# Unbind single 'b' key to prevent conflict with 'ba' chord
config.unbind('b', mode='normal')
config.bind('ba', 'bookmark-add')
config.bind('bA', 'bookmark-add --toggle')
config.bind('gb', 'cmd-set-text -s :bookmark-load ')
config.bind('gB', 'cmd-set-text -s :bookmark-load -t ')

# Quickmarks
config.bind('m', 'quickmark-save')
config.bind('M', 'cmd-set-text -s :quickmark-load ')

# Downloads
config.bind('cd', 'download-clear')
config.bind('gd', 'download-open')

# History
config.bind('ch', 'history-clear')

# Config
config.bind('se', 'config-edit')

# Sessions
config.bind('ss', 'cmd-set-text -s :session-save ')
config.bind('sl', 'cmd-set-text -s :session-load --clear ')

# Dev tools
config.bind('wi', 'devtools')
config.bind('wI', 'devtools-focus')

# Pass mode (for passthrough)
config.bind('<Ctrl-v>', 'mode-enter passthrough')
config.bind('<Escape>', 'mode-leave', mode='passthrough')

# Insert mode
config.bind('<Ctrl-e>', 'open-editor', mode='insert')

# Command mode
config.bind('<Ctrl-p>', 'completion-item-focus prev', mode='command')
config.bind('<Ctrl-n>', 'completion-item-focus next', mode='command')

# Spawn applications
config.bind('gi', 'spawn alacritty -e nvim {url}')

# Bitwarden password manager integration
config.bind(',p', 'spawn --userscript qute-bitwarden')
config.bind(',P', 'spawn --userscript qute-bitwarden --password-only')
config.bind(',u', 'spawn --userscript qute-bitwarden --username-only')
config.bind(',t', 'spawn --userscript qute-bitwarden --totp')

# Print message
print("Qutebrowser config loaded with Noctalia theme")
