local packages = {
    -- MangoWC window manager and related tools
    "bibata-cursor-theme-bin",
    "brightnessctl",
    "cliphist",
    "crystal-dock-bin",
    "grim",
    "mangowc-git",
    "satty",
    "slurp",
    "sway-audio-idle-inhibit-git",
    "swaybg",
    "wl-clip-persist",
    "wl-clipboard",
    "xdg-desktop-portal-wlr",

    -- Default applications
    "fastfetch",
    "fish",
    "foot",
    "kitty",

    -- Shell components
    "dms-shell-git",
    "quickshell",
}

return {
    description = "MangoWC window manager and related tools",
    conflicts = { "bdots-kde" },
    doftiles_sync = true,
    post_install_hook = "scripts/install-mangowc-dotfiles.sh",
    packages = packages,
}
