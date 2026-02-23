local packages = {
    -- MangoWC window manager and related tools
    "bibata-cursor-theme-bin",
    "brightnessctl",
    "cliphist",
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
    "noctalia-shell-git",
    "quickshell-git",
}

return {
    description = "MangoWC window manager and related tools",
    conflicts = { "bdots-kde" },
    doftiles_sync = true,
    post_install_hook = "",
    packages = packages,
}
