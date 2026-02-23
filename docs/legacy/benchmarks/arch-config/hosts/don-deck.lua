---@diagnostic disable: undefined-global -- dcli globals are provided by dcli runtime

return {
    host = "don-deck",
    description = "Packages specific to don-deck",
    import = {},

    enabled_modules = {
        "hardware",
    },

    packages = {},
    exclude = {},
    additional_packages = {},

    flatpak_scope = "user",
    auto_prune = false,

    system_backups = {
        enabled = true,
        tool = "timeshift",
        snapper_config = "root",
    },
}
