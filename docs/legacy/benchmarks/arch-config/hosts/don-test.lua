return {
    host = "don-vm",
    description = "Minimal VM testing environment",
    import = {},

    enabled_modules = {
        "login-managers/sddm-enable",
        "window-managers/bd-niri",
        "dotfiles",
        "hosts-configs/noctalia-ui",
    },

    packages = {},
    exclude = {},

    flatpak_scope = "user",
    auto_prune = false,
    module_processing = "sequential",
    strict_package_order = false,
    aur_helper = "yay",

    config_backups = {
        enabled = false,
    },

    system_backups = {
        enabled = false,
    },

    services = {
        enabled = {},
        disabled = {},
    },
}
