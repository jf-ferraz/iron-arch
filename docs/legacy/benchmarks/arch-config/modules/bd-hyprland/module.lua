local packages = {
    -- Hyprland Wayland compositor
    "hyprland",
    "hypridle",
    "xdg-desktop-portal-hyprland",
    "xdg-desktop-portal-gtk",
    "wl-clipboard",
    "grimblast-git",
    "grim",
    "slurp",
    "nwg-displays",
    "brightnessctl",
    "pavucontrol",
    "network-manager-applet",
    "pyprland",

    -- Build dependencies
    "cmake",
    "meson",
    "cpio",
    "git",
    "gcc",

    -- Default applications
    "nemo",
    "kitty",
    "fastfetch",
    "zen-browser-bin",
    "zed",
    "fish",
    "helix",

    -- Shell components (DankMaterialShell)
    "dms-shell-git",
    "quickshell",
}

return {
    description = "Hyprland Wayland compositor with DankMaterialShell",
    conflicts = { "bdots-kde" },
    post_install_hook = "scripts/install-hyprland-dotfiles.sh",
    hook_behavior = "once",
    dotfiles_sync = false,
    packages = packages,
}
