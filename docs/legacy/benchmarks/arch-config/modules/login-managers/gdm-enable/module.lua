local packages = {
    "gdm",
}

return {
    description = "GDM login manager with auto-enable service",
    conflicts = {
        "login-managers/sddm-enable",
        "login-managers/lightdm-enable",
    },
    post_install_hook = "scripts/enable-gdm.sh",
    packages = packages,
}
