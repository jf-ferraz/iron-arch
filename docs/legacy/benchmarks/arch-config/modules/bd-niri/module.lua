local packages = {
    -- Niri window manager
    "niri",
    "xdg-desktop-portal-gnome",
    "wl-clipboard",
    "wl-clip-persist",
    "cliphist",
    "brightnessctl",
    "fuzzel",
    "sway-audio-idle-inhibit-git",
    "swayidle",

    -- Build dependencies for Niri and related tools
    "cmake",
    "meson",
    "cpio",
    "git",
    "gcc",

    -- Default applications
    "kitty",
    "fastfetch",
    "fish",
    "nemo",

    -- Shell components
    "quickshell-git",
    "noctalia-shell-git",
}

return {
    description = "Niri scrollable-tiling Wayland compositor with defaults",
    conflicts = { "bdots-kde" },
    dotfiles_sync = true,
    packages = packages,
}
