local packages = {
    "sddm",
    "unzip",
}

return {
    description = "SDDM login manager with Catppuccin theme (12-hour time)",
    conflicts = {
        "login-managers/gdm-enable",
        "login-managers/lightdm-enable",
    },
    post_install_hook = "scripts/enable-sddm.sh",
    packages = packages,
}
