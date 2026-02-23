# BSPWM Window Manager Module

This module provides a complete BSPWM (Binary Space Partitioning Window Manager) environment based on [gh0stzk's dotfiles](https://github.com/gh0stzk/dotfiles), featuring 18 beautiful themes with dynamic theme switching, transparency effects, and extensive customization options.

## Features

- **18 Unique Themes**: emilia, jan, aline, andrea, cynthia, isabel, silvia, melissa, pamela, cristina, karla, z0mbi3, brenda, daniela, marisol, h4ck3r, varinka, yael
- **Dynamic Theme Switching**: Change themes on-the-fly without restarting
- **Transparency Effects**: Configurable window transparency with Picom
- **Multiple Bars**: Polybar and Eww widget support
- **Rofi Applets**: Wallpaper selector, network manager, bluetooth, clipboard, and more
- **Scratchpad**: Quick access terminal overlay
- **Lock Screen**: Betterlockscreen integration with wallpaper support
- **Auto-lock**: Automatic screen lock when idle

## Prerequisites

This module requires two additional package repositories to be enabled:

1. **gh0stzk-dotfiles** - Custom themes, icons, and packages
2. **chaotic-aur** - Pre-built AUR packages (eww-git)

## Installation

### Step 1: Run Pre-Install Script

Before applying this module, run the preinstall script to set up required repositories:

```bash
cd /home/don/.config/arch-config/modules/window-managers/bd-bspwm
./preinstall.sh
```

This script will:
- Add the gh0stzk-dotfiles repository to pacman
- Add the chaotic-aur repository with GPG keys
- Install paru (AUR helper) if not present
- Update pacman database

### Step 2: Apply the Module

After running preinstall, apply the module with your configuration management tool or:

```bash
# If using the apply-config.sh script
./apply-config.sh
```

## Required Repositories

### gh0stzk-dotfiles Repository

**Packages provided:**
- `st-gh0stzk` - Customized simple terminal
- `gh0stzk-gtk-themes` - GTK themes matching each rice
- `gh0stzk-cursor-qogirr` - Custom cursor theme
- Icon packs: beautyline, candy, catppuccin-mocha, dracula, glassy, gruvbox-plus-dark, hack, luv, sweet-rainbow, tokyo-night, vimix-white, zafiro, zafiro-purple

**Repository configuration:**
```
[gh0stzk-dotfiles]
SigLevel = Optional TrustAll
Server = http://gh0stzk.github.io/pkgs/x86_64
```

### Chaotic AUR Repository

**Packages provided:**
- `eww-git` - ElKowar's wacky widgets (compiled from git)

**Repository configuration:**
```
[chaotic-aur]
Include = /etc/pacman.d/chaotic-mirrorlist
```

## Package Sources

### Official Arch Repositories (69 packages)

**Window Management:**
- bspwm, sxhkd - Window manager and hotkey daemon
- xdo, xdotool - X11 automation tools
- xsettingsd - X11 settings daemon

**Display & Compositing:**
- picom - Compositor with animations and transparency
- feh - Image viewer and wallpaper setter

**Bars & Notifications:**
- polybar - Status bar
- dunst - Notification daemon
- eww-git - Widgets (from chaotic-aur)

**Terminals:**
- alacritty, kitty - Terminal emulators
- st-gh0stzk - Simple terminal (from gh0stzk repo)

**File Management:**
- thunar - File manager
- tumbler - Thumbnail service
- gvfs-mtp - MTP support for Android devices

**Media & Music:**
- mpd - Music player daemon
- mpc - MPD client
- ncmpcpp - NCurses MPD client
- mpv - Video player

**System Utilities:**
- brightnessctl - Brightness control
- betterlockscreen - Lock screen
- xcolor - Color picker
- xprintidle - Idle time detection

**Fonts:**
- ttf-jetbrains-mono-nerd, ttf-terminus-nerd, ttf-ubuntu-mono-nerd
- ttf-inconsolata, ttf-jetbrains-mono

**ZSH & Shell Tools:**
- zsh, zsh-autosuggestions, zsh-history-substring-search, zsh-syntax-highlighting
- fzf, bat, eza, yazi

**Development Tools:**
- go, rustup, npm, git, base-devel

### AUR Packages (Built via paru)

- `xwinwrap-0.9-bin` - Animated wallpaper support
- `i3lock-color` - Colored lock screen for betterlockscreen
- `fzf-tab-git` - FZF tab completion for ZSH

## Keybindings

### Essential Keybindings

| Keybinding | Action |
|------------|--------|
| `Super + Enter` | Open terminal |
| `Super + Ctrl + Enter` | Open floating terminal |
| `Super + l` | Lock screen |
| `Super + Shift + q` | Close window |
| `Super + q` | Kill window |
| `Super + Space` | Open app launcher (rofi) |
| `Super + Tab` | Window switcher |
| `Super + {1-0}` | Switch to workspace |
| `Super + Shift + {1-0}` | Move window to workspace |
| `Super + Arrow Keys` | Focus window |
| `Super + Shift + Arrow Keys` | Move window |
| `Super + Alt + r` | Reload BSPWM |
| `Super + Escape` | Reload sxhkd keybindings |
| `Alt + Tab` | Cycle windows |

### Theme & Rice Management

| Keybinding | Action |
|------------|--------|
| `Alt + Space` | Theme selector |
| `Super + Alt + w` | Wallpaper selector |

### Function Keys

| Keybinding | Action |
|------------|--------|
| `XF86AudioRaiseVolume` | Volume up |
| `XF86AudioLowerVolume` | Volume down |
| `XF86AudioMute` | Toggle mute |
| `XF86MonBrightnessUp` | Brightness up |
| `XF86MonBrightnessDown` | Brightness down |

## Available Themes (Rices)

Each theme includes:
- Unique color scheme
- Matching GTK theme
- Custom icon pack
- Polybar or Eww bar configuration
- Wallpapers

**Themes available:**
1. **emilia** - Tokyo Night colors
2. **jan** - Gruvbox colors
3. **aline** - Nord colors
4. **andrea** - Catppuccin colors
5. **cynthia** - Dracula colors
6. **isabel** - Everforest colors
7. **silvia** - Rose Pine colors
8. **melissa** - Kanagawa colors
9. **pamela** - Material Ocean colors
10. **cristina** - One Dark colors
11. **karla** - Sweet colors
12. **z0mbi3** - Eww bar with custom widgets
13. **brenda** - Hacker aesthetic
14. **daniela** - Pastel colors
15. **marisol** - Vaporwave aesthetic
16. **h4ck3r** - Matrix/Hacker theme
17. **varinka** - Minimalist light theme
18. **yael** - Deep ocean colors

## Configuration Files

**Main configuration directory:** `~/.config/bspwm/`

**Key files:**
- `bspwmrc` - Main BSPWM configuration
- `config/sxhkdrc` - Keybindings
- `config/picom.conf` - Compositor settings (transparency, animations)
- `rices/<theme>/theme-config.bash` - Per-theme settings
- `bin/` - Helper scripts

## Troubleshooting

### Issue: Repositories not found

**Error:** `target not found: st-gh0stzk` or similar

**Solution:** Run the preinstall script:
```bash
./preinstall.sh
```

### Issue: Permission denied when running preinstall.sh

**Solution:** Make the script executable:
```bash
chmod +x preinstall.sh
./preinstall.sh
```

### Issue: GPG key errors for chaotic-aur

**Solution:** Manually add the key:
```bash
sudo pacman-key --recv-key 3056513887B78AEB --keyserver keyserver.ubuntu.com
sudo pacman-key --lsign-key 3056513887B78AEB
```

### Issue: Packages fail to install

**Check that repositories are enabled:**
```bash
grep -E "^\[gh0stzk-dotfiles|^\[chaotic-aur" /etc/pacman.conf
```

Should output:
```
[gh0stzk-dotfiles]
[chaotic-aur]
```

**Update database:**
```bash
sudo pacman -Syy
```

### Issue: AUR packages fail to build

**Solution:** Ensure paru is installed:
```bash
which paru || ./preinstall.sh
```

### Issue: Transparency not working

**Solution:** Check picom is running:
```bash
pgrep picom || picom --config ~/.config/bspwm/config/picom.conf &
```

### Issue: Theme switching doesn't work

**Solution:** Check that all icon themes are installed:
```bash
pacman -Q | grep gh0stzk-icons
```

If missing, install from gh0stzk-dotfiles repo:
```bash
sudo pacman -Syy
sudo pacman -S gh0stzk-icons-tokyo-night gh0stzk-icons-beautyline
```

### Issue: Lock screen not working

**Solution:** Initialize betterlockscreen cache:
```bash
betterlockscreen -u ~/.config/bspwm/rices/emilia/walls/wall-01.webp
```

### Issue: Eww widgets not showing (z0mbi3 theme)

**Solution:** Ensure eww-git is installed:
```bash
pacman -Q eww-git || sudo pacman -S eww-git
```

### Issue: Auto-lock not working

**Check:** Ensure xprintidle is installed:
```bash
pacman -Q xprintidle || sudo pacman -S xprintidle
```

Check if AutoLock script is running:
```bash
pgrep -f AutoLock
```

## Post-Installation

After installing and logging into BSPWM:

1. **Select a theme:** Press `Alt + Space` to open the theme selector
2. **Set a wallpaper:** Press `Super + Alt + w` for wallpaper selector
3. **Initialize lockscreen:** Run once:
   ```bash
   betterlockscreen -u ~/.config/bspwm/rices/$(cat ~/.config/bspwm/.rice)/walls/
   ```
4. **Configure monitors:** Edit `~/.config/bspwm/bspwmrc` if needed
5. **Enable user services:**
   ```bash
   systemctl --user enable mpd.service
   systemctl --user enable ArchUpdates.timer
   ```

## Resources

- **Original Dotfiles:** https://github.com/gh0stzk/dotfiles
- **Wiki:** https://github.com/gh0stzk/dotfiles/wiki
- **Issues:** Check existing issues on GitHub before creating new ones

## License

This module includes configuration files from gh0stzk/dotfiles which are licensed under GPL-3.0.
