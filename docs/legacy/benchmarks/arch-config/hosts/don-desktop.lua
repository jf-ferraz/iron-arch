return {
    host = "don-desktop",
    description = "",
    sync_sudo = "true",
    auto_commit = "true",
    import = {},

    enabled_modules = {
        "package-mods/main-apps",
        "gaming/gaming-packages",
        "gaming/controller-support",
        "cli-tools/cli-apps",
        "package-mods/content-creation",
        "dev/development",
        "window-managers/bd-niri",
        "window-managers/bd-bspwm",
        "gaming/sunshine",
        "login-managers/sddm-astronaut-enable",
        "cli-tools/webapp-tool",
        "dotfiles",
        "flapak",
        "hardware",
        "ricing/theming-apps",
        "system-packages-don-desktop",
        "virtualization/winapps",
        "virtualization/virt-app",
        "quickshells/inir-shell",
        "dev/flutter",
        "hosts-configs/niri-outputs",
        "hosts-configs/noctalia-ui",
        "declared-packages",
    },

    packages = {
        "qt6-multimedia-ffmpeg",
    },
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
        enabled = {
            "NetworkManager",
            "NetworkManager-dispatcher",
            "NetworkManager-wait-online",
            "ananicy-cpp",
            "avahi-daemon",
            "bluetooth",
            "getty@",
            "sddm",
            "grub-btrfsd",
            "nvidia-hibernate",
            "nvidia-resume",
            "nvidia-suspend",
            "tailscaled",
            "ufw",
        },
        disabled = {
            "greetd",
        },
    },

    enabled_service_profiles = {},

    update_hooks = {
        pre_update = nil,
        post_update = nil,
        behavior = "ask",
    },

    default_apps = {
        scope = "system",
        browser = "org.qutebrowser.qutebrowser.desktop",
        text_editor = "helix",
        file_manager = "nemo",
        terminal = "kitty",
        video_player = "mpv",
        audio_player = "mpv",
        image_viewer = "imv",
        pdf_viewer = "mupdf",
        mime_types = {},
    },

    editor = "helix",
    aur_helper = "yay",
}
