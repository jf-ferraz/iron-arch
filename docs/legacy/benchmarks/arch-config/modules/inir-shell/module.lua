-- iNiR Shell dependencies for Niri compositor
-- This module installs all packages required by iNiR (illogical-impulse for Niri)
-- The post-install hook will clone the repo and run setup automatically

local packages = {
    -- ═══════════════════════════════════════════════════════════
    -- CORE PACKAGES
    -- ═══════════════════════════════════════════════════════════
    "niri",
    "bc",
    "coreutils",
    "cliphist",
    "curl",
    "wget",
    "ripgrep",
    "jq",
    "xdg-user-dirs",
    "rsync",
    "git",
    "wl-clipboard",
    "libnotify",
    "xdg-desktop-portal",
    "xdg-desktop-portal-gtk",
    "xdg-desktop-portal-gnome",
    "polkit",
    "mate-polkit",
    "networkmanager",
    "gnome-keyring",
    "dolphin",
    "foot",
    "gum",

    -- ═══════════════════════════════════════════════════════════
    -- QT6 STACK & QUICKSHELL
    -- ═══════════════════════════════════════════════════════════
    "quickshell-git",
    "qt6-declarative",
    "qt6-base",
    "qt6-svg",
    "qt6-wayland",
    "qt6-5compat",
    "qt6-imageformats",
    "qt6-multimedia",
    "qt6-positioning",
    "qt6-quicktimeline",
    "qt6-sensors",
    "qt6-tools",
    "qt6-translations",
    "qt6-virtualkeyboard",
    "jemalloc",
    "libpipewire",
    "libxcb",
    "wayland",
    "libdrm",
    "mesa",
    "kirigami",
    "kdialog",
    "syntax-highlighting",
    "qt6ct",
    "kde-gtk-config",
    "breeze",
    -- AUR
    "google-breakpad",
    "qt6-avif-image-plugin",

    -- ═══════════════════════════════════════════════════════════
    -- AUDIO STACK
    -- ═══════════════════════════════════════════════════════════
    "pipewire",
    "pipewire-pulse",
    "pipewire-alsa",
    "wireplumber",
    "playerctl",
    "libdbusmenu-gtk3",
    "pavucontrol",

    -- ═══════════════════════════════════════════════════════════
    -- SCREENSHOTS & RECORDING
    -- ═══════════════════════════════════════════════════════════
    "grim",
    "slurp",
    "swappy",
    "tesseract",
    "tesseract-data-eng",
    "wf-recorder",
    "imagemagick",
    "ffmpeg",

    -- ═══════════════════════════════════════════════════════════
    -- INPUT TOOLKIT
    -- ═══════════════════════════════════════════════════════════
    "upower",
    "wtype",
    "ydotool",
    "python-evdev",
    "python-pillow",
    "brightnessctl",
    "ddcutil",
    "geoclue",
    "swayidle",

    -- ═══════════════════════════════════════════════════════════
    -- FONTS & THEMING
    -- ═══════════════════════════════════════════════════════════
    "fontconfig",
    "ttf-dejavu",
    "ttf-liberation",
    "fuzzel",
    "glib2",
    "translate-shell",
    "kvantum",
    -- AUR fonts & theming
    "matugen-bin",
    "ttf-jetbrains-mono-nerd",
    "ttf-material-symbols-variable-git",
    "ttf-readex-pro",
    "ttf-rubik-vf",
    "otf-space-grotesk",
    "ttf-twemoji",
    "capitaine-cursors",
    "hyprpicker",
}

return {
    description = "iNiR shell (illogical-impulse for Niri) - dependencies and setup",
    packages = packages,
    post_install_hook = "scripts/setup-inir.sh",
    hook_behavior = "ask",
}
