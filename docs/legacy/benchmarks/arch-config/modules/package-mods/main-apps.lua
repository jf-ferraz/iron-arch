local packages = {
    -- Terminal
    "kitty",
    "fastfetch",
    -- File Management
    "felix-rs",
    "nemo",
    "thunar",
    -- Comms
    "telegram-desktop",
    "vesktop",
    "discord",
    -- Browsers
    "zen-browser-bin",
    "qutebrowser",
    -- Notes
    "obsidian",
    -- Editor
    "zed",
}

return {
    description = "Main applications and utilities for daily use",
    conflicts = {},
    packages = packages,
}
