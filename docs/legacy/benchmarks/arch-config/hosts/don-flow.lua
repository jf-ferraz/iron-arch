return {
    host = "don-flow",
    description = "Packages specific to don-flow",
    import = {},

    enabled_modules = {
        "window-managers/bd-niri",
        "dotfiles",
        "cli-tools/webapp-tool",
        "package-mods/main-apps",
        "dev/development",
        "dev/flutter",
        "gaming/gaming-packages",
        "cli-tools/cli-apps",
        "hosts-configs/niri-outputs",
        "hosts-configs/noctalia-ui",
        "login-managers/sddm-astronaut-enable",
        "hardware",
        "gaming/moonlight",
    },

    packages = {},
    exclude = {},
    additional_packages = {},

    flatpak_scope = "user",
    auto_prune = true,
    module_processing = "sequential",
    strict_package_order = true,

    config_backups = {
        enabled = true,
        max_backups = 5,
    },

    system_backups = {
        enabled = true,
        backup_on_sync = true,
        backup_on_update = true,
        tool = "snapper",
        snapper_config = "root",
    },

    services = {
        enabled = {},
        disabled = {},
    },

    update_hooks = {
        pre_update = nil,
        post_update = nil,
        behavior = "ask",
    },

    default_apps = {
        scope = "system",
        browser = "org.qutebrowser.qutebrowser.desktop",
        text_editor = "helix",
        video_player = "mpv",
        audio_player = "mpv",
        image_viewer = "imv",
        pdf_viewer = "mupdf",
        mime_types = {},
    },

    editor = "helix",
    aur_helper = "yay",
}
