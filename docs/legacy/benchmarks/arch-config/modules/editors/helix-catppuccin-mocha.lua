local packages = {
    "helix",
}

return {
    description = "Catppuccin Mocha theme for Helix editor",
    conflicts = {
        "editors/helix-catppuccin-frappe",
        "editors/helix-catppuccin-latte",
        "editors/helix-catppuccin-macchiato",
    },
    post_install_hook = "helix-catppuccin-mocha/scripts/install-helix-theme.sh",
    packages = packages,
}
